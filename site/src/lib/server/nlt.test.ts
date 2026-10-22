import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { BOOKS } from '../read/books';
import * as wasm from '../read/wasm/bible.js';
import {
	apiBook,
	apiReference,
	LIMITS,
	MAX_VERSES,
	oneChapterBook,
	parsePassage,
	passageLabel,
	RateLimiter,
	VERSES,
	verseExcerpt
} from './nlt';

/** The API reference a passage asks for, or the error. */
function api(ref: string | null): string {
	const p = parsePassage(ref);
	return p.ok ? apiReference(p.passage) : `error: ${p.error}`;
}

describe('parsePassage', () => {
	it('takes a chapter or part of one, however the reference is written', () => {
		expect(api('John 3')).toBe('John.3');
		expect(api('  john   3 ')).toBe('John.3');
		expect(api('John 3:16')).toBe('John.3.16');
		expect(api('John 3:16-18')).toBe('John.3.16-18');
		expect(api('John 3:16–18')).toBe('John.3.16-18');
		expect(api('John 3:16 - 18')).toBe('John.3.16-18');
		expect(api('John.3.16')).toBe('John.3.16');
		expect(api('Psalm 23')).toBe('Ps.23');
		expect(api('Psalms 119')).toBe('Ps.119');
		expect(api('ps 119:176')).toBe('Ps.119.176');
		expect(api('Song of Solomon 2')).toBe('Song.2');
		expect(api('song-of-solomon 2')).toBe('Song.2');
		expect(api('1 John 4')).toBe('1Jn.4');
		expect(api('1-john 4:8')).toBe('1Jn.4.8');
		expect(api('1john 4')).toBe('1Jn.4');
		expect(api('2 Thessalonians 3')).toBe('2Thes.3');
		expect(api('1 Cor 13:4-7')).toBe('1Cor.13.4-7');
		expect(api('Jude 1')).toBe('Jude.1');
	});

	it('refuses whole books', () => {
		expect(api('John')).toMatch(/all of John/);
		expect(api('Jude')).toMatch(/all of Jude/);
		expect(api('Song of Solomon')).toMatch(/all of Song of Solomon/);
	});

	it('refuses passages that cross chapters, or more than one passage', () => {
		expect(api('John 3-4')).toMatch(/across chapters/);
		expect(api('John 3:16-4:2')).toMatch(/across chapters/);
		expect(api('Genesis 1–50')).toMatch(/across chapters/);
		expect(api('John 3; Romans 8')).toMatch(/One passage at a time/);
		expect(api('John 3, 4')).toMatch(/One passage at a time/);
	});

	it('refuses chapters and verses that don’t exist', () => {
		expect(api('John 22')).toBe('error: John has 21 chapters');
		expect(api('Jude 2')).toBe('error: Jude has 1 chapter');
		expect(api('John 0')).toBe('error: John has 21 chapters');
		expect(api('John 3:0')).toMatch(/No such verses/);
		expect(api('John 3:18-16')).toMatch(/No such verses/);
		expect(api('Psalm 119:178')).toMatch(/No such verses/);
		// The KJV's count and one more: John 3 has 36 verses.
		expect(api('John 3:36')).toBe('John.3.36');
		expect(api('John 3:37')).toBe('John.3.37');
		expect(api('John 3:38')).toMatch(/No such verses/);
		expect(api('John 3:175')).toMatch(/No such verses/);
		expect(api('3 John 1:15')).toBe('3Jn.1.15');
	});

	it('refuses what isn’t a reference', () => {
		expect(api(null)).toMatch(/Ask for a passage/);
		expect(api('')).toMatch(/Ask for a passage/);
		expect(api('Hezekiah 3')).toMatch(/No book matches/);
		expect(api('<script>alert(1)</script>')).toMatch(/No book matches/);
		expect(api('John 3:16abc')).toMatch(/Couldn’t read/);
		expect(api('John 3 '.repeat(20))).toMatch(/too long/);
	});

	it('labels a passage and marks any passage of a one-chapter book', () => {
		const label = (ref: string) => {
			const p = parsePassage(ref);
			return p.ok ? [passageLabel(p.passage), oneChapterBook(p.passage)] : null;
		};
		expect(label('psalms 23')).toEqual(['Psalm 23', false]);
		expect(label('John 3:16-18')).toEqual(['John 3:16–18', false]);
		expect(label('Jude 1')).toEqual(['Jude 1', true]);
		expect(label('Jude 1:1-25')).toEqual(['Jude 1:1–25', true]);
		expect(label('Jude 1:3')).toEqual(['Jude 1:3', true]);
	});

	it('never asks the API for more than 500 verses', () => {
		expect(Math.max(...VERSES.flat()) + 1).toBeLessThanOrEqual(MAX_VERSES);
	});
});

describe('VERSES', () => {
	it('agrees with the core’s verse counts for all 1,189 chapters', () => {
		wasm.initSync({ module: readFileSync('src/lib/read/wasm/bible_bg.wasm') });
		const core = BOOKS.map((b) => Array.from({ length: b.chapters }, (_, i) => wasm.chapterVerses(b.firstChapter + i)));
		expect(VERSES).toEqual(core);
		expect(VERSES.flat().reduce((a, b) => a + b, 0)).toBe(31_102);
	});
});

describe('apiBook', () => {
	it('spells the 66 books as the API does (OSIS, with five exceptions)', () => {
		expect(BOOKS.map(apiBook).join(' ')).toBe(
			'Gen Exod Lev Num Deut Josh Judg Ruth 1Sam 2Sam 1Kgs 2Kgs 1Chr 2Chr Ezra Neh Esth Job Ps Prov ' +
				'Eccl Song Isa Jer Lam Ezek Dan Hos Joel Amos Obad Jonah Mic Nah Hab Zeph Hag Zech Mal Matt ' +
				'Mark Luke John Acts Rom 1Cor 2Cor Gal Eph Phil Col 1Thes 2Thes 1Tim 2Tim Titus Phlm Heb Jas ' +
				'1Pet 2Pet 1Jn 2Jn 3Jn Jude Rev'
		);
	});
});

describe('verseExcerpt', () => {
	// The API's page, with public-domain (KJV) words in it.
	const page = `<!DOCTYPE html><html><head><title>NLT API</title><link rel="stylesheet" href="https://api.nlt.to/x.css"/></head><body>
<div id="bibletext"><section><h2 class="bk_ch_vs_header">John 3:16-17, NLT</h2><verse_export orig="john_3_16" vn="16"><p class="body"><span class="vn">16</span>For God so loved the world.</p></verse_export><verse_export orig="john_3_17" vn="17"><span class="vn">17</span>For God sent not his Son.</verse_export></section></div></body></html>`;

	it('keeps the verses and counts them', () => {
		const e = verseExcerpt(page);
		expect(e?.verses).toBe(2);
		expect(e?.html.startsWith('<verse_export orig="john_3_16"')).toBe(true);
		expect(e?.html.endsWith('</verse_export>')).toBe(true);
		expect(e?.html).not.toContain('NLT API');
		expect(e?.html).not.toContain('bk_ch_vs_header');
	});

	it('finds nothing in an empty page', () => {
		expect(verseExcerpt('')).toBeNull();
		expect(verseExcerpt('<html><body>Not found</body></html>')).toBeNull();
	});
});

describe('RateLimiter', () => {
	const MIN = 60_000;

	it('lets a client make up to the limit, then says how long to wait', () => {
		const r = new RateLimiter([{ max: 3, per: MIN }]);
		const t = 1_000_000;
		expect([r.take('a', t), r.take('a', t + 1), r.take('a', t + 2)]).toEqual([0, 0, 0]);
		expect(r.take('a', t + 10_000)).toBe(50);
		// A refused request isn't counted: the window still ends a minute after it began.
		expect(r.take('a', t + 59_500)).toBe(1);
		expect(r.take('a', t + MIN)).toBe(0);
	});

	it('counts each client on its own', () => {
		const r = new RateLimiter([{ max: 1, per: MIN }]);
		expect(r.take('1.2.3.4', 0)).toBe(0);
		expect(r.take('1.2.3.4', 1)).toBeGreaterThan(0);
		expect(r.take('5.6.7.8', 1)).toBe(0);
	});

	it('applies every limit: a minute’s and a day’s', () => {
		const r = new RateLimiter(LIMITS);
		let t = 0;
		let served = 0;
		// A client asking once a second all day gets 20 a minute until the day's 300 are used.
		for (let i = 0; i < 86_400; i++, t += 1000) if (r.take('a', t) === 0) served++;
		expect(served).toBe(300);
		expect(r.take('a', 86_400_000)).toBe(0);
	});

	it('forgets clients a minute or less after their windows end', () => {
		const r = new RateLimiter([{ max: 1, per: MIN }]);
		r.take('a', 0);
		r.take('b', 30_000);
		expect(r.size).toBe(2);
		// The next request after a minute sweeps: a's window has ended, b's hasn't.
		r.take('c', MIN);
		expect(r.size).toBe(2);
		r.sweep(MIN + 30_000);
		expect(r.size).toBe(1);
	});

	it('forgets clients whose windows have ended once it holds too many', () => {
		const r = new RateLimiter([{ max: 1, per: MIN }], 3);
		for (const ip of ['a', 'b', 'c']) r.take(ip, 0);
		expect(r.size).toBe(3);
		r.take('d', MIN);
		expect(r.size).toBe(1);
		// Still full of live windows: the oldest go.
		for (const ip of ['e', 'f']) r.take(ip, MIN);
		r.take('g', MIN + 1);
		expect(r.size).toBe(3);
		expect(r.take('d', MIN + 2)).toBe(0);
	});
});
