/**
 * bible-core in the page (crates/web, built by scripts/build-web.sh into ./wasm). One `Bible`
 * holds the decoded translations and the study store; translations are fetched from
 * /translations/<id>.bbl (imported ones read from IndexedDB) the first time they're needed. While
 * prerendering, the same code reads the module and the translations from disk (./node.ts).
 */
import { BOOKS, chapterName } from './books';
import { importedBytes } from './imports';
import init, * as wasm from './wasm/bible.js';
import wasmUrl from './wasm/bible_bg.wasm?url';

export type HighlightColor = 'yellow' | 'green' | 'blue' | 'pink' | 'purple' | 'orange';
export const HIGHLIGHT_COLORS: HighlightColor[] = ['yellow', 'green', 'blue', 'pink', 'purple', 'orange'];

export interface TranslationInfo {
	id: string;
	abbrev: string;
	name: string;
	language: string;
	license: string;
	copyright: string;
	year: number | null;
	description: string | null;
	redLetters: boolean;
	headings: boolean;
	/** The decoded text's size in bytes. */
	size: number | null;
	/** Set for a translation imported in this browser: its source format and stored size. */
	imported?: { format: string; bytes: number };
	/** Read live from its publisher's API, a chapter at a time (the NLT: nlt.ts). */
	live?: boolean;
}

export interface Book {
	index: number;
	name: string;
	abbrev: string;
	chapters: number;
	firstChapter: number;
	newTestament: boolean;
}

/** A verse in the canonical grid: book 0–65, chapter and verse from 1. */
export interface VerseRef {
	book: number;
	chapter: number;
	verse: number;
}

export interface Reference {
	book: number;
	chapter: number;
	chapterIndex: number;
	verse: number | null;
	verseEnd: number | null;
	label: string;
}

export interface ContentOptions {
	versePerLine: boolean;
	headings: boolean;
	verseNumbers: boolean;
}

export type ChapterStatus =
	| { type: 'ready' }
	| { type: 'missing' }
	| { type: 'pending' }
	| { type: 'noKey' }
	| { type: 'failed'; message: string };

export type BlockKind =
	| { type: 'prose'; indent: boolean }
	| { type: 'poetry'; level: number }
	| { type: 'right' }
	| { type: 'center' }
	| { type: 'heading'; level: number }
	| { type: 'psalmHeading' }
	| { type: 'psalmTitle' }
	| { type: 'speaker' };

export interface Run {
	text: string;
	/** 0 for headings. */
	verse: number;
	/** An inline verse number, ending in a no-break space. */
	number: boolean;
	red: boolean;
	italic: boolean;
	bold: boolean;
	/** Already upper case; set smaller. */
	smallCaps: boolean;
	/** Where the run's text starts in its verse's text (UTF-16), to place highlights and selections. */
	offset: number;
	/** A space the layout adds: not in the verse's text, `offset` is where it falls. */
	added: boolean;
}

export interface Block {
	kind: BlockKind;
	/** Space above, in pt at a 20 pt reading size. */
	spaceBefore: number;
	/** A verse number set in the margin before the first line. */
	gutter: string | null;
	gutterVerse: number;
	runs: Run[];
}

export interface ChapterContent {
	index: number;
	book: number;
	chapter: number;
	status: ChapterStatus;
	bookName: string;
	bookTitle: string;
	blocks: Block[];
}

export interface VerseContent {
	number: number;
	numberEnd: number;
	title: string | null;
	runs: Run[];
}

export interface SearchHit {
	verse: VerseRef;
	chapterIndex: number;
	reference: string;
	text: string;
	/** [start, end) in UTF-16 code units. */
	matches: [number, number][];
}

export interface SearchPage {
	total: number;
	books: { book: number; name: string; count: number }[];
	hits: SearchHit[];
	suggestion: { query: string; total: number } | null;
}

export type SearchScope = 'all' | 'ot' | 'nt' | number;

export interface VerseMarks {
	verse: number;
	/** The verse's highlight colour (the newest, when parts of it differ). */
	highlight: HighlightColor | null;
	/** What's highlighted of the verse's text, in order. */
	highlights: HighlightRange[];
	note: boolean;
	bookmark: boolean;
}

/** A highlighted stretch of a verse: `[start, end)` in UTF-16 units of its text, as runs' `offset`. */
export interface HighlightRange {
	start: number;
	end: number;
	color: HighlightColor;
}

/** Words chosen in a chapter: from `start` in verse `first` to before `end` in verse `last` (UTF-16 offsets into the verse texts). */
export interface TextSpan {
	first: VerseRef;
	start: number;
	last: VerseRef;
	end: number;
}

export interface Note {
	body: string;
	created: number;
	updated: number;
}

export interface StudyItem {
	verse: VerseRef;
	chapterIndex: number;
	reference: string;
	text: string;
	highlight: HighlightColor | null;
	/** A highlight's place in its verse, for `removeHighlight`. */
	start: number;
	body: string;
	created: number;
	updated: number;
}

export interface Counts {
	highlights: number;
	notes: number;
	bookmarks: number;
}

export interface ReadingSession {
	started: number;
	book: number;
	chapter: number;
	translation: string;
	seconds: number;
	furthest: number;
	verses: number;
}

export interface PlanProgress {
	id: string;
	started: number;
	finished: number | null;
}

export interface PlanInfo {
	id: string;
	title: string;
	meta: string;
	description: string;
	icon: 'calendar' | 'book' | 'quote' | 'history' | 'flame' | 'note';
	days: number;
}

export interface PlanDay {
	label: string;
	passages: { chapterIndex: number; label: string; verses: [number, number] | null }[];
}

export interface ExportInclude {
	highlights: boolean;
	notes: boolean;
	bookmarks: boolean;
	history: boolean;
}

export type CopyFormat = 'plain' | 'reference' | 'markdown';

/** The decoded translations and the study store. */
export class Core {
	readonly bible = new wasm.Bible();
	readonly books: Book[] = wasm.books();
	readonly bundled: string[] = wasm.bundledTranslations();
	readonly #loading = new Map<string, Promise<TranslationInfo>>();
	readonly #info = new Map<string, TranslationInfo>();

	constructor(readonly fetchBytes: (id: string) => Promise<Uint8Array>) {}

	/** Fetches and decodes a translation once; later calls share the first. */
	load(id: string): Promise<TranslationInfo> {
		let p = this.#loading.get(id);
		if (!p) {
			p = this.fetchBytes(id).then((bytes) => {
				const info: TranslationInfo = this.bible.load(bytes);
				this.#info.set(id, info);
				return info;
			});
			p.catch(() => this.#loading.delete(id));
			this.#loading.set(id, p);
		}
		return p;
	}

	/** Takes a translation's file directly (one just imported). */
	add(bytes: Uint8Array): TranslationInfo {
		const info: TranslationInfo = this.bible.load(bytes);
		this.#info.set(info.id, info);
		this.#loading.set(info.id, Promise.resolve(info));
		return info;
	}

	/** Holds a chapter of the NLT (HTML from /api/nlt) in place of the one held before (nlt.ts). */
	loadNlt(index: number, html: string): TranslationInfo {
		const info: TranslationInfo = { ...this.bible.loadNlt(index, html), live: true };
		this.#info.set(info.id, info);
		return info;
	}

	/** Lets a translation's text go (the NLT's chapter, when the reader moves on). */
	unload(id: string) {
		this.bible.unload(id);
		this.#info.delete(id);
		this.#loading.delete(id);
	}

	has(id: string): boolean {
		return this.bible.has(id);
	}

	info(id: string): TranslationInfo | undefined {
		return this.#info.get(id);
	}

	chapter(id: string, index: number, options: ContentOptions): ChapterContent {
		return this.bible.chapter(id, index, options);
	}

	verses(id: string, index: number): VerseContent[] {
		return this.has(id) ? this.bible.verses(id, index) : [];
	}

	verse(id: string, v: VerseRef): VerseContent | null {
		return this.has(id) ? this.bible.verse(id, v) : null;
	}

	search(
		id: string,
		query: string,
		scope: SearchScope,
		wholeWords: boolean,
		firstBook: number | null,
		offset: number,
		limit: number
	): SearchPage {
		return this.bible.search(id, query, String(scope), wholeWords, firstBook ?? -1, offset, limit);
	}

	copyText(id: string, verses: VerseRef[], format: CopyFormat): string | null {
		return this.has(id) ? (this.bible.copyText(id, verses, format) ?? null) : null;
	}

	/** Chosen words for the clipboard, alone or with their reference. */
	copySpan(id: string, span: TextSpan, format: 'plain' | 'reference'): string | null {
		return this.has(id) ? (this.bible.copySpan(id, span, format) ?? null) : null;
	}
}

export const parseReference = (input: string): Reference | null => wasm.parseReference(input);
export const suggestReferences = (input: string, limit: number): Reference[] =>
	wasm.suggestReferences(input, limit);
export const chapterLabel = (index: number): string => wasm.chapterLabel(index);
export const chapterVerses = (index: number): number => wasm.chapterVerses(index);
export const rangeLabel = (from: VerseRef, to: VerseRef): string => wasm.rangeLabel(from, to);
export const planCatalogue = (): PlanInfo[] => wasm.planCatalogue();
export const planDays = (id: string): PlanDay[] => wasm.planDays(id);
export const translationInfo = (bytes: Uint8Array): TranslationInfo => wasm.translationInfo(bytes);

/** The canonical grid's size: Genesis 1 is 0, Revelation 22 is 1188. */
export const CHAPTERS = 1189;
export const DEFAULT_TRANSLATION = 'kjv';
/** John 1: a friendlier first page than Genesis 1 (as on the Mac). */
export const DEFAULT_CHAPTER = 997;

let started: Promise<Core> | undefined;

/** The page's one Core, started on first use. */
export function core(): Promise<Core> {
	started ??= start();
	return started;
}

async function start(): Promise<Core> {
	if (import.meta.env.SSR) {
		const node = await import('./node.js');
		wasm.initSync({ module: node.wasmBytes() });
		const c = new Core(async (id) => node.translationBytes(id));
		const named = Array.from({ length: CHAPTERS }, (_, i) => chapterName(BOOKS, i) === chapterLabel(i));
		if (JSON.stringify(c.books) !== JSON.stringify(BOOKS) || named.includes(false))
			throw new Error('books.ts no longer matches the core’s books');
		return c;
	}
	await init({ module_or_path: wasmUrl });
	const c: Core = new Core(async (id) => {
		if (!c.bundled.includes(id)) return importedBytes(id);
		const res = await fetch(`/translations/${id}.bbl`);
		if (!res.ok) throw new Error(`Couldn't load ${id} (${res.status})`);
		return new Uint8Array(await res.arrayBuffer());
	});
	return c;
}
