/**
 * Search: the query in the toolbar's field, the filters and results in the sidebar, and the
 * navigator under the toolbar stepping through them (the search half of crates/app/src/sidebar.rs).
 * The core searches the whole translation in a few milliseconds; hits are fetched in pages as the
 * result list scrolls.
 */
import { untrack } from 'svelte';
import type { ReadApp } from './app.svelte';
import type { SearchHit, SearchPage, SearchScope } from './core';

/** Hits fetched with the count, then in pages of this many. */
export const FIRST = 2000;
const PAGE = 500;

export class Search {
	query = $state('');
	scope = $state<SearchScope>('all');
	/** Hits in the book being read first, then the rest in canonical order. */
	currentFirst = $state(false);
	/** Whether the toolbar's field has focus (arrow keys then move through the results). */
	fieldFocused = $state(false);
	/** Bumped to ask the toolbar's field for focus. */
	focusRequest = $state(0);
	/** Bumped to scroll the selected result into view. */
	scrollRequest = $state(0);
	/** How long the last search took, in ms (read with `results`). */
	elapsed = 0;
	#selected = $state({ key: '', index: 0 });
	#generation = $state(0);
	#pages = new Map<number, SearchHit[]>();
	#pagesKey = '';
	private readonly app: ReadApp;

	constructor(app: ReadApp) {
		this.app = app;
	}

	get searching(): boolean {
		return this.query.trim() !== '';
	}

	/**
	 * The book "This book first" puts first: the one being read when the search changed, so going
	 * to a hit in another book doesn't reorder the results under it.
	 */
	#preferred = { search: '', book: -1 };

	/** What the results depend on; a change runs the search again and selects the first hit. */
	readonly key = $derived.by(() => {
		const s = this.app.settings;
		const search = JSON.stringify([this.app.textTranslation(), this.query.trim(), this.scope, s.wholeWords, this.currentFirst, this.#generation]);
		if (this.#preferred.search !== search) {
			this.#preferred = { search, book: this.currentFirst ? untrack(() => this.app.hereRef().book) : -1 };
		}
		return `${search}${this.#preferred.book}`;
	});

	/** The count, the books and the first hits, or null with no search (or no text yet). */
	readonly results = $derived.by((): SearchPage | null => {
		const app = this.app;
		const id = app.textTranslation();
		const key = this.key;
		if (!this.searching || !app.core || !app.loaded.has(id)) return null;
		const started = performance.now();
		const page = app.core.search(id, this.query, this.scope, app.settings.wholeWords, this.#first(), 0, FIRST);
		this.elapsed = performance.now() - started;
		this.#pages = new Map([[0, page.hits]]);
		this.#pagesKey = key;
		return page;
	});

	get index(): number {
		return this.#selected.key === this.key ? this.#selected.index : 0;
	}

	get position() {
		const total = this.searching ? (this.results?.total ?? 0) : 0;
		return total ? { index: this.index + 1, total } : null;
	}

	#first(): number | null {
		return this.currentFirst ? this.#preferred.book : null;
	}

	/** The `i`th hit, fetching its page when it isn't in yet. */
	hit(i: number): SearchHit | null {
		const app = this.app;
		const page = this.results;
		if (!page || i < 0 || i >= page.total || !app.core) return null;
		if (this.#pagesKey !== this.key) return null;
		if (i < FIRST) return this.#pages.get(0)?.[i] ?? null;
		const n = Math.floor((i - FIRST) / PAGE);
		let hits = this.#pages.get(n + 1);
		if (!hits) {
			const s = app.settings;
			hits = app.core.search(app.textTranslation(), this.query, this.scope, s.wholeWords, this.#first(), FIRST + n * PAGE, PAGE).hits;
			this.#pages.set(n + 1, hits);
		}
		return hits[i - FIRST - n * PAGE] ?? null;
	}

	/** Selects a hit and goes to it. */
	choose(i: number) {
		const hit = this.hit(i);
		if (!hit) return;
		this.#selected = { key: this.key, index: i };
		this.app.dispatch({ type: 'jump', chapter: hit.chapterIndex, verse: hit.verse.verse, flash: true });
	}

	focus(query?: string) {
		if (query !== undefined) this.query = query;
		this.focusRequest++;
	}

	step(delta: number) {
		const total = this.results?.total ?? 0;
		if (!total) return;
		const i = Math.min(total - 1, Math.max(0, this.index + delta));
		this.choose(i);
		this.scrollRequest++;
	}

	/** Return in the field: the selected hit. */
	open() {
		if (this.results?.total) this.choose(this.index);
	}

	clear() {
		this.query = '';
	}

	invalidate() {
		this.#generation++;
	}
}

/** A hit's snippet: where it starts (a few words before the first match) and its runs. */
export interface Snippet {
	/** Whether it starts mid-verse (shown after "…"). */
	cut: boolean;
	runs: { text: string; hit: boolean }[];
}

/** Matches separated only by spaces read as one ("love one another"). */
function joined(text: string, matches: [number, number][]): [number, number][] {
	const out: [number, number][] = [];
	for (const [s, e] of matches) {
		const last = out[out.length - 1];
		if (last && /^ *$/.test(text.slice(last[1], s))) last[1] = e;
		else out.push([s, e]);
	}
	return out;
}

/** Five words before the first match, or the verse's start when the match is in its first seven. */
function kwicStart(text: string, first: number | undefined): number {
	if (first === undefined) return 0;
	const starts: number[] = [];
	for (let i = 0; i < first; i++) {
		if (!/\s/.test(text[i]) && (i === 0 || /\s/.test(text[i - 1]))) starts.push(i);
	}
	return starts.length <= 7 ? 0 : starts[starts.length - 5];
}

export function snippet(hit: SearchHit): Snippet {
	const { text } = hit;
	const ranges = joined(text, hit.matches);
	const start = kwicStart(text, ranges[0]?.[0]);
	const runs: Snippet['runs'] = [];
	let pos = start;
	for (const [s, e] of ranges) {
		if (s < start) continue;
		if (s > pos) runs.push({ text: text.slice(pos, s), hit: false });
		runs.push({ text: text.slice(s, e), hit: true });
		pos = e;
	}
	if (pos < text.length) runs.push({ text: text.slice(pos), hit: false });
	return { cut: start > 0, runs };
}

/** "1,234". */
export const group = (n: number) => n.toLocaleString('en-US');
