import { json } from '@sveltejs/kit';
import { NLT_CREDIT, NLT_SITE } from '#lib/read/nlt.js';
import {
	apiReference,
	LIMITS,
	MAX_VERSES,
	oneChapterBook,
	parsePassage,
	passageLabel,
	RateLimiter,
	verseExcerpt
} from '#lib/server/nlt.js';
import type { RequestHandler } from './$types';

/**
 * The NLT for the reader (src/lib/read/nlt.ts): /api/nlt?ref=John+3 asks Tyndale's NLT API for
 * one chapter or less with the site's key (NLT_API_KEY, which never leaves the server) and returns
 * its verses' HTML with Tyndale's credit line. Browsers are told to keep nothing; Vercel's edge
 * keeps a chapter for a day, but nothing from a one-chapter book, which could be the whole book.
 */
export const prerender = false;

const API = 'https://api.nlt.to/api/passages';
const limiter = new RateLimiter(LIMITS);
// Clients are also forgotten while no requests come in (the timer doesn't keep the process alive).
setInterval(() => limiter.sweep(), 60_000).unref?.();

const headers = { 'cache-control': 'no-store', 'x-robots-tag': 'noindex' };
const refuse = (status: number, error: string, extra: Record<string, string> = {}) =>
	json({ error }, { status, headers: { ...headers, ...extra } });

export const GET: RequestHandler = async ({ url, getClientAddress }) => {
	let client = 'unknown';
	try {
		client = getClientAddress();
	} catch {
		// No address (a local preview): everyone shares one limit.
	}
	const wait = limiter.take(client);
	if (wait) return refuse(429, 'Too many requests', { 'retry-after': String(wait) });

	const parsed = parsePassage(url.searchParams.get('ref'));
	if (!parsed.ok) return refuse(400, parsed.error);
	const passage = parsed.passage;

	const key = process.env.NLT_API_KEY?.trim();
	if (!key) {
		console.error('/api/nlt: NLT_API_KEY is not set');
		return refuse(503, 'The NLT isn’t available here');
	}

	let page: string;
	try {
		const ref = encodeURIComponent(apiReference(passage));
		const res = await fetch(`${API}?ref=${ref}&version=NLT&key=${encodeURIComponent(key)}`, {
			signal: AbortSignal.timeout(10_000)
		});
		if (!res.ok) {
			console.error(`/api/nlt: the NLT API answered ${res.status}`);
			return refuse(res.status === 429 ? 503 : 502, 'Tyndale’s NLT API didn’t answer');
		}
		page = await res.text();
	} catch (e) {
		// Only the error's name: a message could carry the request's URL, and with it the key.
		console.error(`/api/nlt: couldn’t reach the NLT API (${e instanceof Error ? e.name : 'error'})`);
		return refuse(502, 'Couldn’t reach Tyndale’s NLT API');
	}

	const excerpt = verseExcerpt(page);
	if (!excerpt) return refuse(502, `No text came back for ${passageLabel(passage)}`);
	if (excerpt.verses > MAX_VERSES) return refuse(502, 'More verses came back than may be quoted');

	return json(
		{
			reference: passageLabel(passage),
			verses: excerpt.verses,
			html: excerpt.html,
			credit: NLT_CREDIT,
			link: NLT_SITE
		},
		{
			headers: {
				...headers,
				// Only Vercel reads this (and drops it from the response). A day keeps popular
				// chapters off Tyndale's daily limit without keeping text for long.
				...(oneChapterBook(passage) ? {} : { 'vercel-cdn-cache-control': 'max-age=86400' })
			}
		}
	);
};
