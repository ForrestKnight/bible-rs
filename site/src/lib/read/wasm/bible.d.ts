/* tslint:disable */
/* eslint-disable */

/**
 * The translations the page has loaded, decoded, and the study store.
 */
export class Bible {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Chapter `index` (0–1188) as blocks of styled runs, ready to typeset. A translation that
     * isn't loaded yet gives a `pending` chapter; one without the chapter a `missing` one.
     */
    chapter(translation: string, index: number, options: any): any;
    /**
     * Forgets reading history from `since` on (all of it for 0).
     */
    clearReading(since: number): void;
    /**
     * Words chosen in `translation` for the clipboard: "plain" (just the words) or "reference"
     * (`“so loved the world” — John 3:16 (KJV)`). Null when the translation hasn't them.
     */
    copySpan(translation: string, span: any, format: string): string | undefined;
    /**
     * Verses as text for the clipboard or a share sheet. `format` is "plain" (the verses
     * joined), "reference" (`“text” — John 3:16 (KJV)`, or a reference line and numbered verses
     * for several) or "markdown" (a quote with its reference). Null when none of the verses are
     * in the translation.
     */
    copyText(translation: string, verses: any, format: string): string | undefined;
    counts(): any;
    /**
     * Erases every highlight, note, bookmark, plan and the reading history.
     */
    eraseStudy(): void;
    /**
     * The study data as "markdown" or (re-importable, by the Mac app too) "json", with verse
     * text from `translation`. `include` picks highlights, notes, bookmarks and history
     * (reading sessions and plans); left out, it's the first three.
     */
    exportStudy(format: string, translation: string, include: any): string;
    finishPlan(id: string, at?: number | null): void;
    /**
     * Increments whenever study data changes.
     */
    generation(): number;
    /**
     * Whether a translation is loaded.
     */
    has(translation: string): boolean;
    /**
     * Colours a stretch of text as read in `translation` (`{first, start, last, end}`, UTF-16
     * offsets into the verse texts), or with null clears it: over the highlights there, which it
     * replaces where they overlap.
     */
    highlightSpan(translation: string, span: any, color?: string | null): void;
    /**
     * Merges a JSON export in (entries for the same verse are overwritten); returns how many
     * highlights, notes and bookmarks it held.
     */
    importStudy(json: string): any;
    /**
     * Keeps one chapter of the NLT, read from the HTML Tyndale's API returned (through the site's
     * `/api/nlt`), in place of any chapter of it kept before: Tyndale's terms allow quoting up to
     * 500 verses and never a whole book, so the page holds one chapter at a time and stores none.
     */
    loadNlt(index: number, html: string): any;
    /**
     * Decodes a `.bbl` file (≈10 ms for a whole Bible) and keeps it under its id, which is
     * returned with its metadata.
     */
    load(bytes: Uint8Array): any;
    /**
     * Highlights, notes and bookmarks on the verses of a chapter (as numbered in `translation`,
     * or the KJV's numbering when it isn't loaded).
     */
    marks(translation: string, index: number): any;
    constructor();
    note(verse: any): any;
    /**
     * The passages ticked off in a plan, as `[day, passage index]` pairs, in order.
     */
    planReads(id: string): any;
    /**
     * Plans started, newest first.
     */
    plans(): any;
    /**
     * Builds a translation's search text now (≈30 ms), so the first query doesn't wait for it.
     */
    prepareSearch(translation: string): void;
    /**
     * Reading sessions that started at or after `since` (Unix seconds), newest first.
     */
    readingSince(since: number): any;
    /**
     * Saves a reading session, replacing an earlier save of the same session.
     */
    recordReading(session: any): void;
    /**
     * Removes one highlight of a verse: the one a study item's `start` names.
     */
    removeHighlight(verse: any, start: number): void;
    /**
     * Replaces the study store with a `snapshot`.
     */
    restore(json: string): void;
    /**
     * Searches a translation: every word must appear (as a word prefix unless `whole_words`),
     * `"quoted phrases"` in order, `-words` excluded. `scope` is "all", "ot", "nt" or a book
     * index. With `first_book` (a book index, or -1) that book's hits come first, the rest in
     * canonical order. Returns `limit` hits from `offset` on.
     */
    search(translation: string, query: string, scope: string, whole_words: boolean, first_book: number, offset: number, limit: number): any;
    /**
     * Bookmarks a verse (with a label, possibly empty) or, with null, removes its bookmark.
     */
    setBookmark(verse: any, label?: string | null): void;
    /**
     * Sets or (with null) clears a verse's highlight: "yellow", "green", "blue", "pink",
     * "purple" or "orange".
     */
    setHighlight(verse: any, color?: string | null): void;
    /**
     * Saves a note; an empty one deletes it.
     */
    setNote(verse: any, body: string): void;
    setPlanRead(id: string, day: number, passage: number, read: boolean): void;
    /**
     * Everything in the study store, for the page to keep (`restore` reads it back).
     */
    snapshot(): string;
    /**
     * The colour every letter of a stretch of text has, if one does.
     */
    spanColor(translation: string, span: any): string | undefined;
    /**
     * Starts (or restarts) a plan on the day containing `started`, forgetting earlier progress.
     */
    startPlan(id: string, started: number): void;
    stopPlan(id: string): void;
    /**
     * "bookmarks" and "notes" in canonical order, "highlights" the newest first, each with its
     * verse's text in `translation`.
     */
    studyItems(kind: string, translation: string): any;
    /**
     * Bookmarks a verse or removes its bookmark; returns whether it's bookmarked now.
     */
    toggleBookmark(verse: any): boolean;
    /**
     * Lets a translation go (the NLT's chapter, once the reader moves on).
     */
    unload(translation: string): void;
    /**
     * One verse as a paragraph, or null when the translation hasn't it (comparisons).
     */
    verse(translation: string, verse: any): any;
    /**
     * The verses of a chapter, each as one paragraph (parallel columns). Empty when the
     * translation hasn't the chapter.
     */
    verses(translation: string, index: number): any;
}

/**
 * The 66 books of the canonical grid.
 */
export function books(): any;

/**
 * The bundled translations' ids, in the order they're offered.
 */
export function bundledTranslations(): string[];

/**
 * "John 3", "Psalm 23".
 */
export function chapterLabel(index: number): string;

/**
 * The number of verses a chapter has in the KJV's numbering.
 */
export function chapterVerses(index: number): number;

/**
 * The best reading of a typed reference ("jn 3:16", "Rom 8", "revalation 21"), or null.
 */
export function parseReference(input: string): any;

/**
 * The reading plans on offer, in order.
 */
export function planCatalogue(): any;

/**
 * A plan's days, each with its passages.
 */
export function planDays(id: string): any;

/**
 * "John 3:16", "John 3:16–18", "John 3:16–4:2".
 */
export function rangeLabel(from: any, to: any): string;

/**
 * Readings of a typed reference, the best first.
 */
export function suggestReferences(input: string, limit: number): any;

/**
 * A translation's metadata from its file's header, without decoding the text.
 */
export function translationInfo(bytes: Uint8Array): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_bible_free: (a: number, b: number) => void;
    readonly bible_chapter: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly bible_clearReading: (a: number, b: number, c: number) => void;
    readonly bible_copySpan: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly bible_copyText: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly bible_counts: (a: number, b: number) => void;
    readonly bible_eraseStudy: (a: number, b: number) => void;
    readonly bible_exportStudy: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly bible_finishPlan: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly bible_generation: (a: number) => number;
    readonly bible_has: (a: number, b: number, c: number) => number;
    readonly bible_highlightSpan: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly bible_importStudy: (a: number, b: number, c: number, d: number) => void;
    readonly bible_load: (a: number, b: number, c: number, d: number) => void;
    readonly bible_loadNlt: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_marks: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_new: () => number;
    readonly bible_note: (a: number, b: number, c: number) => void;
    readonly bible_planReads: (a: number, b: number, c: number, d: number) => void;
    readonly bible_plans: (a: number, b: number) => void;
    readonly bible_prepareSearch: (a: number, b: number, c: number, d: number) => void;
    readonly bible_readingSince: (a: number, b: number, c: number) => void;
    readonly bible_recordReading: (a: number, b: number, c: number) => void;
    readonly bible_removeHighlight: (a: number, b: number, c: number, d: number) => void;
    readonly bible_restore: (a: number, b: number, c: number, d: number) => void;
    readonly bible_search: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => void;
    readonly bible_setBookmark: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_setHighlight: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_setNote: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_setPlanRead: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly bible_snapshot: (a: number, b: number) => void;
    readonly bible_spanColor: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_startPlan: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_stopPlan: (a: number, b: number, c: number, d: number) => void;
    readonly bible_studyItems: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly bible_toggleBookmark: (a: number, b: number, c: number) => void;
    readonly bible_unload: (a: number, b: number, c: number) => void;
    readonly bible_verse: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly bible_verses: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly books: (a: number) => void;
    readonly bundledTranslations: (a: number) => void;
    readonly chapterLabel: (a: number, b: number) => void;
    readonly chapterVerses: (a: number) => number;
    readonly parseReference: (a: number, b: number, c: number) => void;
    readonly planCatalogue: (a: number) => void;
    readonly planDays: (a: number, b: number, c: number) => void;
    readonly rangeLabel: (a: number, b: number, c: number) => void;
    readonly suggestReferences: (a: number, b: number, c: number, d: number) => void;
    readonly translationInfo: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
