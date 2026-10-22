/**
 * The rules /api/nlt keeps (src/routes/api/nlt/+server.ts). Tyndale's permission terms let
 * non-commercial apps quote the NLT up to 500 verses, never a complete book, with their credit
 * line. So the proxy answers one chapter or less a request, refuses whole books and passages that
 * cross chapters, and a reader's browser can ask for a limited number of chapters a minute and a
 * day.
 */
import { BOOKS, bookSlug } from '../read/books';
import type { Book } from '../read/core';

/** The most verses one request to the API may ask for (Tyndale's limit with a key). */
export const MAX_VERSES = 500;

/**
 * Each book's chapters' verse counts, as the KJV numbers them (the core's `chapterVerses`; a test
 * checks they agree). Psalm 119's 176 are the most, so every chapter fits one request.
 */
export const VERSES: number[][] = [
	[31, 25, 24, 26, 32, 22, 24, 22, 29, 32, 32, 20, 18, 24, 21, 16, 27, 33, 38, 18, 34, 24, 20, 67, 34, 35, 46, 22, 35, 43, 55, 32, 20, 31, 29, 43, 36, 30, 23, 23, 57, 38, 34, 34, 28, 34, 31, 22, 33, 26], // Genesis
	[22, 25, 22, 31, 23, 30, 25, 32, 35, 29, 10, 51, 22, 31, 27, 36, 16, 27, 25, 26, 36, 31, 33, 18, 40, 37, 21, 43, 46, 38, 18, 35, 23, 35, 35, 38, 29, 31, 43, 38], // Exodus
	[17, 16, 17, 35, 19, 30, 38, 36, 24, 20, 47, 8, 59, 57, 33, 34, 16, 30, 37, 27, 24, 33, 44, 23, 55, 46, 34], // Leviticus
	[54, 34, 51, 49, 31, 27, 89, 26, 23, 36, 35, 16, 33, 45, 41, 50, 13, 32, 22, 29, 35, 41, 30, 25, 18, 65, 23, 31, 40, 16, 54, 42, 56, 29, 34, 13], // Numbers
	[46, 37, 29, 49, 33, 25, 26, 20, 29, 22, 32, 32, 18, 29, 23, 22, 20, 22, 21, 20, 23, 30, 25, 22, 19, 19, 26, 68, 29, 20, 30, 52, 29, 12], // Deuteronomy
	[18, 24, 17, 24, 15, 27, 26, 35, 27, 43, 23, 24, 33, 15, 63, 10, 18, 28, 51, 9, 45, 34, 16, 33], // Joshua
	[36, 23, 31, 24, 31, 40, 25, 35, 57, 18, 40, 15, 25, 20, 20, 31, 13, 31, 30, 48, 25], // Judges
	[22, 23, 18, 22], // Ruth
	[28, 36, 21, 22, 12, 21, 17, 22, 27, 27, 15, 25, 23, 52, 35, 23, 58, 30, 24, 42, 15, 23, 29, 22, 44, 25, 12, 25, 11, 31, 13], // 1 Samuel
	[27, 32, 39, 12, 25, 23, 29, 18, 13, 19, 27, 31, 39, 33, 37, 23, 29, 33, 43, 26, 22, 51, 39, 25], // 2 Samuel
	[53, 46, 28, 34, 18, 38, 51, 66, 28, 29, 43, 33, 34, 31, 34, 34, 24, 46, 21, 43, 29, 53], // 1 Kings
	[18, 25, 27, 44, 27, 33, 20, 29, 37, 36, 21, 21, 25, 29, 38, 20, 41, 37, 37, 21, 26, 20, 37, 20, 30], // 2 Kings
	[54, 55, 24, 43, 26, 81, 40, 40, 44, 14, 47, 40, 14, 17, 29, 43, 27, 17, 19, 8, 30, 19, 32, 31, 31, 32, 34, 21, 30], // 1 Chronicles
	[17, 18, 17, 22, 14, 42, 22, 18, 31, 19, 23, 16, 22, 15, 19, 14, 19, 34, 11, 37, 20, 12, 21, 27, 28, 23, 9, 27, 36, 27, 21, 33, 25, 33, 27, 23], // 2 Chronicles
	[11, 70, 13, 24, 17, 22, 28, 36, 15, 44], // Ezra
	[11, 20, 32, 23, 19, 19, 73, 18, 38, 39, 36, 47, 31], // Nehemiah
	[22, 23, 15, 17, 14, 14, 10, 17, 32, 3], // Esther
	[22, 13, 26, 21, 27, 30, 21, 22, 35, 22, 20, 25, 28, 22, 35, 22, 16, 21, 29, 29, 34, 30, 17, 25, 6, 14, 23, 28, 25, 31, 40, 22, 33, 37, 16, 33, 24, 41, 30, 24, 34, 17], // Job
	[6, 12, 8, 8, 12, 10, 17, 9, 20, 18, 7, 8, 6, 7, 5, 11, 15, 50, 14, 9, 13, 31, 6, 10, 22, 12, 14, 9, 11, 12, 24, 11, 22, 22, 28, 12, 40, 22, 13, 17, 13, 11, 5, 26, 17, 11, 9, 14, 20, 23, 19, 9, 6, 7, 23, 13, 11, 11, 17, 12, 8, 12, 11, 10, 13, 20, 7, 35, 36, 5, 24, 20, 28, 23, 10, 12, 20, 72, 13, 19, 16, 8, 18, 12, 13, 17, 7, 18, 52, 17, 16, 15, 5, 23, 11, 13, 12, 9, 9, 5, 8, 28, 22, 35, 45, 48, 43, 13, 31, 7, 10, 10, 9, 8, 18, 19, 2, 29, 176, 7, 8, 9, 4, 8, 5, 6, 5, 6, 8, 8, 3, 18, 3, 3, 21, 26, 9, 8, 24, 13, 10, 7, 12, 15, 21, 10, 20, 14, 9, 6], // Psalms
	[33, 22, 35, 27, 23, 35, 27, 36, 18, 32, 31, 28, 25, 35, 33, 33, 28, 24, 29, 30, 31, 29, 35, 34, 28, 28, 27, 28, 27, 33, 31], // Proverbs
	[18, 26, 22, 16, 20, 12, 29, 17, 18, 20, 10, 14], // Ecclesiastes
	[17, 17, 11, 16, 16, 13, 13, 14], // Song of Solomon
	[31, 22, 26, 6, 30, 13, 25, 22, 21, 34, 16, 6, 22, 32, 9, 14, 14, 7, 25, 6, 17, 25, 18, 23, 12, 21, 13, 29, 24, 33, 9, 20, 24, 17, 10, 22, 38, 22, 8, 31, 29, 25, 28, 28, 25, 13, 15, 22, 26, 11, 23, 15, 12, 17, 13, 12, 21, 14, 21, 22, 11, 12, 19, 12, 25, 24], // Isaiah
	[19, 37, 25, 31, 31, 30, 34, 22, 26, 25, 23, 17, 27, 22, 21, 21, 27, 23, 15, 18, 14, 30, 40, 10, 38, 24, 22, 17, 32, 24, 40, 44, 26, 22, 19, 32, 21, 28, 18, 16, 18, 22, 13, 30, 5, 28, 7, 47, 39, 46, 64, 34], // Jeremiah
	[22, 22, 66, 22, 22], // Lamentations
	[28, 10, 27, 17, 17, 14, 27, 18, 11, 22, 25, 28, 23, 23, 8, 63, 24, 32, 14, 49, 32, 31, 49, 27, 17, 21, 36, 26, 21, 26, 18, 32, 33, 31, 15, 38, 28, 23, 29, 49, 26, 20, 27, 31, 25, 24, 23, 35], // Ezekiel
	[21, 49, 30, 37, 31, 28, 28, 27, 27, 21, 45, 13], // Daniel
	[11, 23, 5, 19, 15, 11, 16, 14, 17, 15, 12, 14, 16, 9], // Hosea
	[20, 32, 21], // Joel
	[15, 16, 15, 13, 27, 14, 17, 14, 15], // Amos
	[21], // Obadiah
	[17, 10, 10, 11], // Jonah
	[16, 13, 12, 13, 15, 16, 20], // Micah
	[15, 13, 19], // Nahum
	[17, 20, 19], // Habakkuk
	[18, 15, 20], // Zephaniah
	[15, 23], // Haggai
	[21, 13, 10, 14, 11, 15, 14, 23, 17, 12, 17, 14, 9, 21], // Zechariah
	[14, 17, 18, 6], // Malachi
	[25, 23, 17, 25, 48, 34, 29, 34, 38, 42, 30, 50, 58, 36, 39, 28, 27, 35, 30, 34, 46, 46, 39, 51, 46, 75, 66, 20], // Matthew
	[45, 28, 35, 41, 43, 56, 37, 38, 50, 52, 33, 44, 37, 72, 47, 20], // Mark
	[80, 52, 38, 44, 39, 49, 50, 56, 62, 42, 54, 59, 35, 35, 32, 31, 37, 43, 48, 47, 38, 71, 56, 53], // Luke
	[51, 25, 36, 54, 47, 71, 53, 59, 41, 42, 57, 50, 38, 31, 27, 33, 26, 40, 42, 31, 25], // John
	[26, 47, 26, 37, 42, 15, 60, 40, 43, 48, 30, 25, 52, 28, 41, 40, 34, 28, 41, 38, 40, 30, 35, 27, 27, 32, 44, 31], // Acts
	[32, 29, 31, 25, 21, 23, 25, 39, 33, 21, 36, 21, 14, 23, 33, 27], // Romans
	[31, 16, 23, 21, 13, 20, 40, 13, 27, 33, 34, 31, 13, 40, 58, 24], // 1 Corinthians
	[24, 17, 18, 18, 21, 18, 16, 24, 15, 18, 33, 21, 14], // 2 Corinthians
	[24, 21, 29, 31, 26, 18], // Galatians
	[23, 22, 21, 32, 33, 24], // Ephesians
	[30, 30, 21, 23], // Philippians
	[29, 23, 25, 18], // Colossians
	[10, 20, 13, 18, 28], // 1 Thessalonians
	[12, 17, 18], // 2 Thessalonians
	[20, 15, 16, 16, 25, 21], // 1 Timothy
	[18, 26, 17, 22], // 2 Timothy
	[16, 15, 15], // Titus
	[25], // Philemon
	[14, 18, 19, 16, 14, 20, 28, 13, 28, 39, 40, 29, 25], // Hebrews
	[27, 26, 18, 17, 20], // James
	[25, 25, 22, 19, 14], // 1 Peter
	[21, 22, 18], // 2 Peter
	[10, 29, 24, 21, 21], // 1 John
	[13], // 2 John
	[14], // 3 John
	[25], // Jude
	[20, 29, 22, 11, 14, 17, 17, 13, 21, 11, 19, 17, 18, 20, 8, 21, 18, 24, 21, 15, 27, 21], // Revelation
];

/**
 * The last verse a reference may name: the KJV's count and one more, since the NLT sometimes
 * numbers one more verse (3 John has 15, the KJV 14), as `online/nlt.rs` allows for.
 */
export const lastVerse = (book: Book, chapter: number) => VERSES[book.index][chapter - 1] + 1;

/** A passage in one chapter: the whole chapter, or verses `[first, last]` of it. */
export interface Passage {
	book: Book;
	chapter: number;
	verses: [number, number] | null;
}

export type Parsed = { ok: true; passage: Passage } | { ok: false; error: string };

/** Book names as typed: names, URL slugs, abbreviations, with or without spaces. */
const NAMES = new Map<string, Book>();
for (const book of BOOKS) {
	const osis = book.abbrev.replace(/ /g, '');
	for (const name of [book.name, bookSlug(book), book.abbrev, osis, apiBook(book)])
		NAMES.set(name.toLowerCase().replace(/[\s-]+/g, ' '), book);
}
NAMES.set('psalm', BOOKS[18]);

const fail = (error: string): Parsed => ({ ok: false, error });

/**
 * Reads a reference ("John 3", "John 3:16-18", "1 John 4", "John.3.16") as a passage the proxy
 * may fetch: one chapter or part of one.
 */
export function parsePassage(input: string | null): Parsed {
	const ref = (input ?? '').trim().replace(/\s+/g, ' ');
	if (!ref) return fail('Ask for a passage, like ?ref=John+3');
	if (ref.length > 60) return fail('That reference is too long');
	if (/[;,&]/.test(ref)) return fail('One passage at a time');
	const m = /^((?:[1-3][ -]?)?[a-z]+(?:[ -][a-z]+)*)\.?(?: ?(.*))?$/i.exec(ref);
	const book = m && NAMES.get(m[1].toLowerCase().replace(/[\s-]+/g, ' '));
	if (!m || !book) return fail(`No book matches “${ref}”`);
	const rest = (m[2] ?? '').replace(/\s/g, '');
	if (!rest) return fail(`That’s all of ${book.name}: the NLT comes one chapter or less at a time`);
	const range = /^(\d+)(?:[:.](\d+)(?:[-–](\d+))?)?$/.exec(rest);
	if (!range) {
		return /^\d+(?:[:.]\d+)?[-–]\d+(?:[:.]\d+)?$/.test(rest)
			? fail('That runs across chapters: the NLT comes one chapter or less at a time')
			: fail(`Couldn’t read “${ref}”`);
	}
	const chapter = Number(range[1]);
	if (chapter < 1 || chapter > book.chapters)
		return fail(`${book.name} has ${book.chapters} chapter${book.chapters === 1 ? '' : 's'}`);
	if (!range[2]) return { ok: true, passage: { book, chapter, verses: null } };
	const first = Number(range[2]);
	const last = range[3] ? Number(range[3]) : first;
	if (first < 1 || last < first || last > lastVerse(book, chapter))
		return fail(`No such verses in ${book.name} ${chapter}`);
	return { ok: true, passage: { book, chapter, verses: [first, last] } };
}

/** The API's name for a book: OSIS, except five it spells its own way (online/nlt.rs `api_book`). */
export function apiBook(book: Book): string {
	const osis = book.abbrev.replace(/ /g, '');
	return { '1Thess': '1Thes', '2Thess': '2Thes', '1John': '1Jn', '2John': '2Jn', '3John': '3Jn' }[osis] ?? osis;
}

/** The passage as the API takes it: "John.3", "John.3.16-18". */
export function apiReference(p: Passage): string {
	const [first, last] = p.verses ?? [0, 0];
	const verses = !p.verses ? '' : last > first ? `.${first}-${last}` : `.${first}`;
	return `${apiBook(p.book)}.${p.chapter}${verses}`;
}

/** "John 3", "John 3:16–18". */
export function passageLabel(p: Passage): string {
	const name = p.book.name === 'Psalms' ? 'Psalm' : p.book.name;
	if (!p.verses) return `${name} ${p.chapter}`;
	const [first, last] = p.verses;
	return `${name} ${p.chapter}:${first}${last > first ? `–${last}` : ''}`;
}

/** A passage from a one-chapter book (Jude), which may be the whole book: shown, but never kept,
 *  even at the edge. */
export const oneChapterBook = (p: Passage) => p.book.chapters === 1;

/**
 * The verses of the API's page and their count: from the first `<verse_export>` to the end of the
 * last, without the page's head and the passage's title. Null when no verse came back.
 */
export function verseExcerpt(html: string): { html: string; verses: number } | null {
	const start = html.indexOf('<verse_export');
	const end = html.lastIndexOf('</verse_export>');
	if (start < 0 || end < start) return null;
	const excerpt = html.slice(start, end + '</verse_export>'.length);
	return { html: excerpt, verses: excerpt.match(/<verse_export[\s>]/g)?.length ?? 0 };
}

export interface Limit {
	max: number;
	/** The window, in ms. */
	per: number;
}

/**
 * Requests per key (a client's address) in fixed windows, each limit counted separately. It lives
 * in one function instance, so it slows a single client down rather than capping the site;
 * Tyndale's own daily limit on the key is the backstop. A client is forgotten once all its windows
 * have ended (`sweep`, at most a minute later while requests come in).
 */
export class RateLimiter {
	readonly #windows = new Map<string, { start: number; count: number }[]>();
	#swept = 0;

	constructor(
		readonly limits: Limit[],
		/** Past this many clients, ones whose windows have all ended are forgotten (then the oldest). */
		readonly maxKeys = 50_000
	) {}

	/** Counts a request; returns 0 when it's allowed, else the seconds until it would be. */
	take(key: string, now = Date.now()): number {
		if (now - this.#swept >= 60_000) this.sweep(now);
		let windows = this.#windows.get(key);
		if (!windows) {
			if (this.#windows.size >= this.maxKeys) this.#prune();
			windows = this.limits.map(() => ({ start: now, count: 0 }));
			this.#windows.set(key, windows);
		}
		let wait = 0;
		this.limits.forEach((limit, i) => {
			const w = windows[i];
			if (now - w.start >= limit.per) windows[i] = { start: now, count: 0 };
			else if (w.count >= limit.max) wait = Math.max(wait, w.start + limit.per - now);
		});
		if (wait > 0) return Math.ceil(wait / 1000);
		for (const w of windows) w.count++;
		return 0;
	}

	get size(): number {
		return this.#windows.size;
	}

	/** Forgets the clients whose windows have all ended. */
	sweep(now = Date.now()) {
		this.#swept = now;
		for (const [key, windows] of this.#windows) {
			if (this.limits.every((limit, i) => now - windows[i].start >= limit.per)) this.#windows.delete(key);
		}
	}

	/** Makes room when every client still has a window open: the oldest go. */
	#prune() {
		for (const key of this.#windows.keys()) {
			if (this.#windows.size < this.maxKeys) break;
			this.#windows.delete(key);
		}
	}
}

/** A reader turning pages: 20 chapters a minute, 300 a day, per address. */
export const LIMITS: Limit[] = [
	{ max: 20, per: 60_000 },
	{ max: 300, per: 86_400_000 }
];
