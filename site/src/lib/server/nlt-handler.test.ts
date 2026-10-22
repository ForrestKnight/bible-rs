import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GET } from '../../routes/api/nlt/+server';

// The API's page for a passage, with public-domain (KJV) words in it.
const PAGE = `<html><head><title>NLT API</title></head><body><div id="bibletext"><section>
<h2 class="bk_ch_vs_header">John 3, NLT</h2><verse_export orig="john_3_16" vn="16"><p class="body"><span class="vn">16</span>For God so loved the world.</p></verse_export></section></div></body></html>`;

let upstream: ReturnType<typeof vi.fn>;
let address = 0;

beforeEach(() => {
	upstream = vi.fn(async () => new Response(PAGE, { status: 200 }));
	vi.stubGlobal('fetch', upstream);
	vi.stubEnv('NLT_API_KEY', 'test-key');
	vi.spyOn(console, 'error').mockImplementation(() => {});
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.unstubAllEnvs();
	vi.restoreAllMocks();
});

/** A request from its own address, so the rate limit doesn't carry over between tests. */
function get(ref: string, ip = `10.0.0.${++address}`) {
	const url = new URL(`https://simplymybible.com/api/nlt?ref=${ref}`);
	return GET({ url, getClientAddress: () => ip } as unknown as Parameters<typeof GET>[0]) as Promise<Response>;
}

describe('/api/nlt', () => {
	it('asks the API for a chapter with the key, and returns its verses with the credit line', async () => {
		const res = await get('John+3');
		expect(res.status).toBe(200);
		expect(upstream).toHaveBeenCalledTimes(1);
		const called = new URL(String(upstream.mock.calls[0][0]));
		expect(called.origin + called.pathname).toBe('https://api.nlt.to/api/passages');
		expect(Object.fromEntries(called.searchParams)).toEqual({ ref: 'John.3', version: 'NLT', key: 'test-key' });
		const body = await res.json();
		expect(body.reference).toBe('John 3');
		expect(body.verses).toBe(1);
		expect(body.html).toMatch(/^<verse_export/);
		expect(body.credit).toMatch(/Tyndale House Foundation/);
		expect(JSON.stringify(body)).not.toContain('test-key');
		expect(res.headers.get('cache-control')).toBe('no-store');
		expect(res.headers.get('vercel-cdn-cache-control')).toBe('max-age=86400');
		expect(res.headers.get('x-robots-tag')).toBe('noindex');
	});

	it('keeps nothing of a one-chapter book at the edge, whole or in part', async () => {
		for (const ref of ['Jude+1', 'Jude+1:1-25', 'Jude+1:3']) {
			const res = await get(ref);
			expect(res.status).toBe(200);
			expect(res.headers.get('vercel-cdn-cache-control')).toBeNull();
			expect(res.headers.get('cache-control')).toBe('no-store');
		}
	});

	it('refuses whole books, passages across chapters and references that don’t exist, without calling the API', async () => {
		for (const ref of ['John', 'Jude', 'John+3-4', 'John+3:16-4:2', 'John+3%3B+Romans+8', 'Hezekiah+3', 'John+22', 'John+3:38', 'John+3:175', '']) {
			const res = await get(ref);
			expect(res.status, ref).toBe(400);
			expect((await res.json()).error).toBeTruthy();
		}
		expect(upstream).not.toHaveBeenCalled();
	});

	it('turns a client away after 20 requests in a minute, without calling the API', async () => {
		const statuses = [];
		for (let i = 0; i < 21; i++) statuses.push((await get('John', '192.0.2.1')).status);
		expect(statuses.slice(0, 20).every((s) => s === 400)).toBe(true);
		const res = await get('John+3', '192.0.2.1');
		expect(res.status).toBe(429);
		expect(Number(res.headers.get('retry-after'))).toBeGreaterThan(0);
		expect(upstream).not.toHaveBeenCalled();
	});

	it('says so without the key, and never shows the key when the API fails', async () => {
		vi.stubEnv('NLT_API_KEY', '');
		expect((await get('John+3')).status).toBe(503);
		vi.stubEnv('NLT_API_KEY', 'test-key');
		upstream.mockImplementation(async () => {
			throw new TypeError('fetch failed: https://api.nlt.to/api/passages?key=test-key');
		});
		const res = await get('John+3');
		expect(res.status).toBe(502);
		expect(await res.text()).not.toContain('test-key');
		expect(JSON.stringify(vi.mocked(console.error).mock.calls)).not.toContain('test-key');
	});

	it('refuses an empty page from the API', async () => {
		upstream.mockImplementation(async () => new Response('', { status: 200 }));
		expect((await get('John+3')).status).toBe(502);
	});
});
