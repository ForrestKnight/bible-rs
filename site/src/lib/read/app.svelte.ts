/**
 * The reader's state and everything it can do: the web counterpart of the Mac app's `BibleApp`
 * (crates/app/src/app.rs). Components read state from here and change it with `dispatch`, so
 * shortcuts, menus, the palette and buttons share one code path, as on the Mac (actions.rs).
 */
import { getContext, setContext } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import { Search } from './search.svelte';
import { deleteImported, importedInfos, storeImported, type ImportResult } from './imports';
import { importer } from './overlays/importer.svelte';
import {
	core as startCore,
	chapterLabel,
	chapterVerses,
	planDays,
	type ChapterContent,
	type ContentOptions,
	type Core,
	type Counts,
	type HighlightColor,
	type Note,
	type PlanProgress,
	type ReadingSession,
	type StudyItem,
	type TextSpan,
	type TranslationInfo,
	type VerseContent,
	type VerseMarks,
	type VerseRef,
	HIGHLIGHT_COLORS,
	CHAPTERS,
	DEFAULT_TRANSLATION
} from './core';
import {
	contentOptions,
	defaultSettings,
	defaultTypography,
	loadSettings,
	saveSettings,
	STUDY_KEY,
	THEMES,
	type Accent,
	type Settings,
	type SidebarTab,
	type Theme
} from './settings';
import { dayStart } from './time';

export type Page = 'reader' | 'translations' | 'bookmarks' | 'highlights' | 'notes' | 'history' | 'plans';
export type SettingsTab = 'reading' | 'appearance' | 'translations' | 'shortcuts' | 'data' | 'about';
export type StudyKind = 'bookmarks' | 'highlights' | 'notes';

/** A place in the text: a chapter index and a verse. */
export interface Loc {
	chapter: number;
	verse: number;
}

export interface Toast {
	id: number;
	text: string;
	/** A failure that can be tried again; it stays until it's dismissed or retried. */
	retry?: () => void;
}

export type Action =
	| { type: 'jump'; chapter: number; verse?: number; flash?: boolean }
	| { type: 'back' }
	| { type: 'forward' }
	| { type: 'prevChapter' }
	| { type: 'nextChapter' }
	| { type: 'setTranslation'; id: string }
	| { type: 'translationNumber'; n: number }
	| { type: 'toggleParallel' }
	| { type: 'openInParallel'; then: Action }
	| { type: 'compareVerse'; verse: VerseRef }
	| { type: 'addParallel'; id: string }
	| { type: 'replaceParallel'; index: number; id: string }
	| { type: 'removeParallel'; id: string }
	| { type: 'highlight'; color: HighlightColor | null }
	| { type: 'editNote'; verse: VerseRef }
	| { type: 'editSelectedNote' }
	| { type: 'toggleBookmark' }
	| { type: 'toggleBookmarkAt'; verse: VerseRef }
	| { type: 'removeBookmark'; verse: VerseRef }
	/** One highlight of a verse: the one a study item's `start` names. */
	| { type: 'removeHighlight'; verse: VerseRef; start: number }
	| { type: 'removeNote'; verse: VerseRef }
	| { type: 'saveNote'; verse: VerseRef; body: string }
	| { type: 'startPlan'; id: string }
	| { type: 'stopPlan'; id: string }
	| { type: 'setPlanRead'; plan: string; day: number; passage: number; read: boolean }
	| { type: 'finishPlan'; id: string; at: number | null }
	| { type: 'setPlanReminder'; on: boolean }
	| { type: 'setAccent'; accent: Accent }
	| { type: 'setReduceMotion'; on: boolean }
	| { type: 'eraseStudy' }
	| { type: 'toggleFocus' }
	| { type: 'textFormat' }
	| { type: 'clearReading'; since: number }
	| { type: 'copyText' }
	| { type: 'copy' }
	| { type: 'openVerseMenu'; x: number; y: number }
	| { type: 'shareImage' }
	| { type: 'compare' }
	| { type: 'clearSelection' }
	| { type: 'selectNext' }
	| { type: 'selectPrev' }
	| { type: 'openPalette' }
	| { type: 'openSearch'; query?: string }
	| { type: 'searchStep'; delta: number }
	| { type: 'clearSearch' }
	| { type: 'toggleSidebar' }
	| { type: 'sidebarTab'; tab: SidebarTab }
	| { type: 'openSettings'; tab: SettingsTab }
	| { type: 'fontSize'; delta: number }
	| { type: 'fontReset' }
	| { type: 'setTheme'; theme: Theme }
	| { type: 'cycleTheme' }
	| { type: 'toggleMode' }
	| { type: 'toggleRedLetters' }
	| { type: 'export'; kind: 'json' | 'markdown' }
	| { type: 'importStudy' }
	| { type: 'restoreBackup' }
	| { type: 'importTranslation'; folder?: boolean }
	| { type: 'removeTranslation'; id: string }
	| { type: 'openPage'; page: Page }
	| { type: 'showShortcuts' }
	| { type: 'scrollLines'; n: number }
	| { type: 'scrollPages'; n: number }
	| { type: 'toast'; text: string };

/** What the reader component offers the app (registered while it's mounted). */
export interface ReaderHandle {
	/** Shows a chapter from its start, or a verse a little below the top. */
	jump(chapter: number, verse: number | undefined, flash: boolean): void;
	/** Scrolls by `dy` px, animated. */
	scrollBy(dy: number): void;
	/** A page's scroll distance. */
	page(): number;
	/** How far the current chapter is scrolled past its start, in px. */
	offsetInChapter(): number;
	/** Scrolls just enough to show a verse. */
	ensureVisible(v: VerseRef): void;
	/** Where the selection is on screen (its washes together), for popovers. */
	selectionRect(): DOMRect | null;
	/** Where a verse's note marker is on screen, for the note popover. */
	markerRect(v: VerseRef): DOMRect | null;
}

/** Sessions shorter than this aren't history (reading.rs). */
const MIN_SECONDS = 8;
const MAX_GAP = 120;
const SAVE_EVERY = 20;
const TOAST_MS = 4000;
const BACKUPS_KEY = 'bible.read.backups';
const BACKUPS_KEPT = 10;

/** Words chosen in the reader: the stretch, the parallel column, and the translation they were
 *  counted in (they go when that column shows another). */
export interface Words {
	span: TextSpan;
	column: number;
	translation: string;
}

export const refEq = (a: VerseRef, b: VerseRef) =>
	a.book === b.book && a.chapter === b.chapter && a.verse === b.verse;
export const refCmp = (a: VerseRef, b: VerseRef) =>
	a.book - b.book || a.chapter - b.chapter || a.verse - b.verse;

/** "John 3:16". */
export function refLabel(v: VerseRef, books: { firstChapter: number }[]) {
	const index = books[v.book].firstChapter + v.chapter - 1;
	return `${chapterLabel(index)}:${v.verse}`;
}

export const today = () => {
	const d = new Date();
	d.setHours(0, 0, 0, 0);
	return Math.floor(d.getTime() / 1000);
};

export const now = () => Math.floor(Date.now() / 1000);

export class ReadApp {
	/** The core, once the module and the reading translation are in. */
	core = $state.raw<Core | null>(null);
	coreError = $state<string | null>(null);
	/** Every translation in the library (the bundled ones, from the page's data). */
	translations = $state.raw<TranslationInfo[]>([]);
	/** Translations decoded and ready. */
	readonly loaded = new SvelteSet<string>();
	/** The defaults until the saved settings are adopted after hydration (`adopt`). */
	settings = $state<Settings>(defaultSettings());
	/** Where the reader is: the chapter at the reading line and its first verse in view. */
	here = $state<Loc>({ chapter: 0, verse: 1 });
	/** The furthest verse in view in the current chapter (reading history). */
	furthest = $state(0);
	selection = $state.raw<[VerseRef, VerseRef] | null>(null);
	/** Words chosen in the reader (a text selection) and their parallel column; the verses they
	 *  touch are the selection, and they go when it changes to anything else. */
	words = $state.raw<Words | null>(null);
	/** The reader was last pressed by touch (or a pen): the highlight bar shows for its selection. */
	touch = $state(false);
	/** Chosen words kept while the pointer is pressed outside the text (a menu, the highlight bar),
	 *  which may take the page's selection away. */
	heldWords: Words | null = null;
	history = $state<{ back: Loc[]; forward: Loc[] }>({ back: [], forward: [] });
	page = $state<Page>('reader');
	/** The translation selected on the Translations page (an import selects what it added). */
	translationPick = $state<string | null>(null);
	palette = $state(false);
	settingsTab = $state<SettingsTab | null>(null);
	shortcuts = $state(false);
	verseMenu = $state.raw<{ x: number; y: number } | null>(null);
	/** The verse whose note popover is open. */
	note = $state.raw<VerseRef | null>(null);
	/** Compare as a popover on the selected verse, or as the inspector beside parallel columns. */
	compare = $state.raw<VerseRef | null>(null);
	panel = $state.raw<VerseRef | null>(null);
	share = $state.raw<VerseRef[] | null>(null);
	importSheet = $state(false);
	textPopover = $state(false);
	focus = $state(false);
	onboarding = $state(false);
	toasts = $state<Toast[]>([]);
	/** A long task under way (an import), shown after the toasts until it ends. */
	busy = $state<string | null>(null);
	/** Text scrolled beneath the toolbar (it then takes the toolbar material). */
	underToolbar = $state(false);
	/** The same for the open page. */
	pageScrolled = $state(false);
	/** The toolbar's field on a page, filtering it. */
	pageQuery = $state('');
	/** Bumped on every study change; reads below depend on it. */
	studyGen = $state(0);
	/** Development only: states `?demo=` asks components to show (see demo.ts). */
	demo = $state<Record<string, string>>({});
	reader: ReaderHandle | null = null;
	readonly search = new Search(this);
	/** The first chapter's names, from the page, until the core can name chapters itself. */
	#initNames: { index: number; label: string; short: string };

	#content = new Map<string, ChapterContent>();
	#verses = new Map<string, VerseContent[]>();
	#toastId = 0;
	#saveStudy: ReturnType<typeof setTimeout> | undefined;
	#studyDirty = false;
	/** The day the plan reminder last showed. */
	#reminded: number | null = null;
	#session: ReadingSession | null = null;
	#seconds = 0;
	#last: number | null = null;
	#savedAt = 0;

	constructor(init: {
		translations: TranslationInfo[];
		translation: string;
		chapter: number;
		content: ChapterContent;
		label: string;
		short: string;
	}) {
		this.translations = init.translations;
		this.settings.translation = init.translation;
		this.here = { chapter: init.chapter, verse: 1 };
		this.#content.set(this.#key(init.translation, init.chapter, contentOptions(defaultTypography())), init.content);
		this.#initNames = { index: init.chapter, label: init.label, short: init.short };
	}

	/** A result navigator shows under the toolbar while a search has results. */
	get resultBand(): boolean {
		return this.search.position !== null;
	}

	/** "Romans 8", or "Rom 8" `short` (compact toolbars). */
	chapterName(index: number, short = false): string {
		const books = this.core?.books;
		if (!books) {
			const n = this.#initNames;
			return index === n.index ? (short ? n.short : n.label) : '';
		}
		if (!short) return chapterLabel(index);
		const book = books.findLast((b) => b.firstChapter <= index) ?? books[0];
		return `${book.abbrev} ${index - book.firstChapter + 1}`;
	}

	// ---------------------------------------------------------------- startup

	/**
	 * Takes the saved settings once the page has hydrated (and the translations imported in this
	 * browser); a translation in the URL wins.
	 */
	adopt(urlTranslation: string | null) {
		this.translations = [...this.translations.filter((t) => !t.imported), ...importedInfos()];
		const saved = loadSettings();
		if (urlTranslation) saved.translation = urlTranslation;
		else if (!this.info(saved.translation)) saved.translation = this.settings.translation;
		saved.parallel = saved.parallel.filter((id) => this.info(id));
		this.settings = saved;
		this.onboarding = !saved.onboarded && !urlTranslation;
	}

	/** Starts the core: the module, the saved study and the translations being read. */
	async start() {
		try {
			const core = await startCore();
			try {
				const saved = localStorage.getItem(STUDY_KEY);
				if (saved) core.bible.restore(saved);
			} catch (e) {
				console.error('Saved study unreadable', e);
			}
			this.core = core;
			this.studyGen++;
			await Promise.all(this.columns().map((id) => this.ensure(id)));
		} catch (e) {
			this.coreError = e instanceof Error ? e.message : String(e);
		}
	}

	/** Loads a translation if it isn't yet. */
	async ensure(id: string): Promise<boolean> {
		const core = this.core;
		if (!core || !this.translations.some((t) => t.id === id)) return false;
		if (this.loaded.has(id)) return true;
		try {
			await core.load(id);
			this.loaded.add(id);
			return true;
		} catch (e) {
			const offline = typeof navigator !== 'undefined' && !navigator.onLine;
			const abbrev = this.info(id)?.abbrev ?? id.toUpperCase();
			this.toastNow(
				offline ? `Couldn't load ${abbrev}: you're offline` : `Couldn't load ${abbrev}: ${e instanceof Error ? e.message : e}`,
				() => void this.ensure(id)
			);
			return false;
		}
	}

	/** Adds a converted translation to the library and reads it (app.rs `install_import`). */
	async installImported(r: ImportResult) {
		const info: TranslationInfo = { ...r.info, imported: { format: r.format, bytes: r.bytes.length } };
		try {
			await storeImported(info, r.bytes);
		} catch (e) {
			this.toastNow(`Couldn't save the import: ${e instanceof Error ? e.message : e}`);
			return;
		}
		this.#forget(info.id);
		if (this.core) {
			this.core.add(r.bytes);
			this.loaded.add(info.id);
		}
		this.translations = [...this.translations, info];
		this.translationPick = info.id;
		this.dispatch({ type: 'setTranslation', id: info.id });
		this.toastNow(`Imported ${info.name} (${r.verses.toLocaleString('en-US')} verses)`);
	}

	async removeImported(id: string) {
		const info = this.info(id);
		if (!info?.imported) {
			this.toastNow("The bundled translations can't be removed");
			return;
		}
		try {
			await deleteImported(id);
		} catch (e) {
			this.toastNow(`Couldn't remove ${info.abbrev}: ${e instanceof Error ? e.message : e}`);
			return;
		}
		const s = this.settings;
		this.translations = this.translations.filter((t) => t.id !== id);
		if (s.translation === id) this.dispatch({ type: 'setTranslation', id: DEFAULT_TRANSLATION });
		s.parallel = s.parallel.filter((p) => p !== id);
		if (this.translationPick === id) this.translationPick = null;
		this.loaded.delete(id);
		this.#forget(id);
		this.toastNow(`Removed ${info.abbrev}`);
	}

	/** Drops a translation's chapters from the caches (an import removed, or replaced under its id). */
	#forget(id: string) {
		for (const cache of [this.#content, this.#verses]) {
			for (const key of cache.keys()) if (key.startsWith(`${id}:`)) cache.delete(key);
		}
	}

	// ---------------------------------------------------------------- reading

	info(id: string): TranslationInfo | undefined {
		return this.translations.find((t) => t.id === id);
	}

	/** The translations shown: the reading one, then up to three parallel columns. */
	columns(): string[] {
		const s = this.settings;
		const cols = [s.translation];
		if (s.parallelOn) {
			for (const id of s.parallel) {
				if (cols.length < 4 && !cols.includes(id) && this.info(id)) cols.push(id);
			}
			if (cols.length === 1) {
				const other = this.translations.find((t) => t.id !== cols[0]);
				if (other) cols.push(other.id);
			}
		}
		return cols;
	}

	contentOptions(): ContentOptions {
		return contentOptions(this.settings.typo);
	}

	#key(id: string, index: number, o: ContentOptions) {
		return `${id}:${index}:${+o.versePerLine}${+o.headings}${+o.verseNumbers}`;
	}

	/** A chapter's blocks, or null while its translation loads. */
	content(id: string, index: number): ChapterContent | null {
		const o = this.contentOptions();
		const ready = this.loaded.has(id);
		const key = this.#key(id, index, o);
		const cached = this.#content.get(key);
		if (cached) return cached;
		if (!ready || !this.core) return null;
		const c = this.core.chapter(id, index, o);
		this.#content.set(key, c);
		return c;
	}

	/** A chapter's verses as paragraphs (parallel columns, compare, share). */
	verses(id: string, index: number): VerseContent[] | null {
		if (!this.loaded.has(id) || !this.core) return null;
		const key = `${id}:${index}`;
		let v = this.#verses.get(key);
		if (!v) {
			v = this.core.verses(id, index);
			this.#verses.set(key, v);
		}
		return v;
	}

	selectedRefs(): VerseRef[] {
		const sel = this.selection;
		if (!sel) return [];
		const [a, b] = sel;
		if (a.book !== b.book || a.chapter !== b.chapter) return [a];
		const out: VerseRef[] = [];
		for (let v = a.verse; v <= b.verse; v++) out.push({ book: a.book, chapter: a.chapter, verse: v });
		return out;
	}

	/** The chosen words, while their verses are the selection and their column shows the
	 *  translation they were counted in. */
	wordSpan(): Words | null {
		const w = this.words;
		const sel = this.selection;
		if (!w || !sel || this.columns()[w.column] !== w.translation) return null;
		return refEq(sel[0], w.span.first) && refEq(sel[1], w.span.last) ? w : null;
	}

	/** Chooses words (`null` lets go of them, and of their verses). */
	selectWords(words: Words | null) {
		if (words) {
			this.words = words;
			this.selection = [words.span.first, words.span.last];
		} else if (this.wordSpan()) {
			this.words = null;
			this.selection = null;
		}
	}

	/** The colour all of the selection has (the chosen words, else the selected verses whole). */
	selectionColor(): HighlightColor | null {
		void this.studyGen;
		const refs = this.selectedRefs();
		const w = this.wordSpan();
		const span = w?.span ?? (refs.length ? { first: refs[0], start: 0, last: refs[refs.length - 1], end: 0xffffffff } : null);
		const id = w?.translation ?? this.settings.translation;
		if (!span || !this.core?.has(id)) return null;
		return (this.core.bible.spanColor(id, span) as HighlightColor | undefined) ?? null;
	}

	hereRef(): VerseRef {
		return this.refAt(this.here.chapter, this.here.verse);
	}

	refAt(index: number, verse: number): VerseRef {
		const books = this.core?.books;
		if (books) {
			const book = books.findLast((b) => b.firstChapter <= index) ?? books[0];
			return { book: book.index, chapter: index - book.firstChapter + 1, verse };
		}
		return { book: 0, chapter: 1, verse };
	}

	chapterIndex(v: VerseRef): number {
		return (this.core?.books[v.book]?.firstChapter ?? 0) + v.chapter - 1;
	}

	selectionOrLocation(): VerseRef[] {
		const sel = this.selectedRefs();
		return sel.length ? sel : [this.hereRef()];
	}

	/** Selects a verse; with `extend`, from the selection's start to it (same chapter only). */
	select(v: VerseRef, extend: boolean) {
		// Clicking the verse of chosen words selects it whole, rather than letting go of it.
		const words = this.wordSpan();
		this.words = null;
		const sel = this.selection;
		if (sel && extend && sel[0].book === v.book && sel[0].chapter === v.chapter) {
			const a = sel[0];
			this.selection = refCmp(v, a) < 0 ? [v, a] : [a, v];
		} else if (sel && !extend && !words && refEq(sel[0], v) && refEq(sel[1], v)) {
			this.selection = null;
		} else {
			this.selection = [v, v];
		}
	}

	setSelection(v: VerseRef | null) {
		this.words = null;
		this.selection = v ? [v, v] : null;
	}

	#stepSelection(delta: number) {
		const core = this.core;
		const id = this.settings.translation;
		if (!core || !this.loaded.has(id)) return;
		const sel = this.selection;
		const base = sel ? (delta > 0 ? sel[1] : sel[0]) : this.hereRef();
		let target: VerseRef | null = null;
		if (!sel) {
			target = base;
		} else {
			const index = this.chapterIndex(base);
			const numbers = this.verses(id, index)?.map((v) => v.number) ?? [];
			const at = numbers.indexOf(base.verse);
			const next = at + delta;
			if (at >= 0 && next >= 0 && next < numbers.length) {
				target = { ...base, verse: numbers[next] };
			} else if (at >= 0) {
				const other = index + delta;
				if (other >= 0 && other < CHAPTERS) {
					const nums = this.verses(id, other)?.map((v) => v.number) ?? [];
					const n = delta > 0 ? nums[0] : nums[nums.length - 1];
					if (n) target = this.refAt(other, n);
				}
			}
		}
		if (target) {
			this.setSelection(target);
			this.reader?.ensureVisible(target);
		}
	}

	#jump(chapter: number, verse: number | undefined, flash: boolean) {
		this.#pushHistory();
		const s = this.settings;
		this.reader?.jump(chapter, verse, flash && s.flashJumps && !s.reduceMotion);
	}

	#pushHistory() {
		const here = { ...this.here };
		const back = this.history.back;
		const last = back[back.length - 1];
		if (!last || last.chapter !== here.chapter || last.verse !== here.verse) {
			back.push(here);
			if (back.length > 200) back.shift();
		}
		this.history.forward = [];
	}

	// ---------------------------------------------------------------- study

	marks(id: string, index: number): Map<number, VerseMarks> {
		void this.studyGen;
		// Highlights are placed in the translation's text: again once it has loaded.
		void this.loaded.has(id);
		const out = new Map<number, VerseMarks>();
		if (!this.core) return out;
		for (const m of this.core.bible.marks(id, index) as VerseMarks[]) out.set(m.verse, m);
		return out;
	}

	noteFor(v: VerseRef): Note | null {
		void this.studyGen;
		return this.core?.bible.note(v) ?? null;
	}

	studyItems(kind: StudyKind): StudyItem[] {
		void this.studyGen;
		if (!this.core || !this.loaded.has(this.settings.translation)) return [];
		return this.core.bible.studyItems(kind, this.settings.translation);
	}

	counts(): Counts {
		void this.studyGen;
		return this.core?.bible.counts() ?? { highlights: 0, notes: 0, bookmarks: 0 };
	}

	plans(): PlanProgress[] {
		void this.studyGen;
		return this.core?.bible.plans() ?? [];
	}

	planReads(id: string): Set<string> {
		void this.studyGen;
		const reads: [number, number][] = this.core?.bible.planReads(id) ?? [];
		return new Set(reads.map(([d, p]) => `${d}:${p}`));
	}

	readingSince(since: number): ReadingSession[] {
		void this.studyGen;
		return this.core?.bible.readingSince(since) ?? [];
	}

	highlightName(c: HighlightColor): string {
		const i = HIGHLIGHT_COLORS.indexOf(c);
		const name = this.settings.highlightNames[i]?.trim();
		return name || c[0].toUpperCase() + c.slice(1);
	}

	/** Runs a study write, then saves; a failure becomes a toast. */
	study<T>(write: (core: Core) => T, failure = "Couldn't save"): T | undefined {
		const core = this.core;
		if (!core) return undefined;
		try {
			const out = write(core);
			this.studyChanged();
			return out;
		} catch (e) {
			this.toastNow(`${failure}: ${e instanceof Error ? e.message : e}`);
			return undefined;
		}
	}

	studyChanged() {
		this.studyGen++;
		this.#studyDirty = true;
		clearTimeout(this.#saveStudy);
		this.#saveStudy = setTimeout(() => this.saveStudyNow(), 300);
	}

	/** Saves the study if it changed here, since each save replaces what other tabs saved. */
	saveStudyNow() {
		clearTimeout(this.#saveStudy);
		if (!this.core || !this.#studyDirty) return;
		try {
			localStorage.setItem(STUDY_KEY, this.core.bible.snapshot());
			this.#studyDirty = false;
		} catch (e) {
			this.toastNow(`Couldn't save your study: ${e instanceof Error ? e.message : e}`);
		}
	}

	/**
	 * Another tab saved its study: take it, or with changes here still to save, add it to them
	 * (what that tab deleted then comes back, rather than losing what this one added).
	 */
	studySaved(json: string | null) {
		const core = this.core;
		if (!core || json === null) return;
		try {
			if (this.#studyDirty) core.bible.importStudy(json);
			else core.bible.restore(json);
			this.studyGen++;
		} catch (e) {
			console.error('Saved study unreadable', e);
		}
	}

	/** Backups kept in the browser (the Mac keeps files): newest first. */
	backups(): { at: number; json: string }[] {
		try {
			return JSON.parse(localStorage.getItem(BACKUPS_KEY) ?? '[]');
		} catch {
			return [];
		}
	}

	backupNow(): boolean {
		if (!this.core) return false;
		const list = this.backups();
		list.unshift({ at: now(), json: this.core.bible.snapshot() });
		try {
			localStorage.setItem(BACKUPS_KEY, JSON.stringify(list.slice(0, BACKUPS_KEPT)));
			return true;
		} catch {
			return false;
		}
	}

	/** Downloads the study as JSON or Markdown, with what Settings › Data includes. */
	exportStudy(kind: 'json' | 'markdown') {
		const core = this.core;
		if (!core) return;
		const body = core.bible.exportStudy(kind, this.settings.translation, this.settings.exportInclude);
		download(`bible-notes.${kind === 'json' ? 'json' : 'md'}`, body, kind === 'json' ? 'application/json' : 'text/markdown');
		this.toastNow(`Exported bible-notes.${kind === 'json' ? 'json' : 'md'}`);
	}

	importStudyText(json: string) {
		const counts = this.study((c) => c.bible.importStudy(json) as Counts, 'Import failed');
		if (counts) this.toastNow(`Imported ${counts.highlights} highlights, ${counts.notes} notes, ${counts.bookmarks} bookmarks`);
	}

	// ---------------------------------------------------------------- reading history

	/** Called about once a second with where the reader is (reading.rs's tick). */
	tick(focused: boolean) {
		this.#remindPlans();
		const core = this.core;
		const id = this.settings.translation;
		if (!core || !this.loaded.has(id) || this.page !== 'reader') return;
		const time = performance.now() / 1000;
		const { chapter } = this.here;
		const ref = this.refAt(chapter, 1);
		const s = this.#session;
		if (!s || s.book !== ref.book || s.chapter !== ref.chapter || s.translation !== id) {
			this.finishReading();
			const verses = this.verses(id, chapter) ?? [];
			this.#session = {
				started: now(),
				book: ref.book,
				chapter: ref.chapter,
				translation: id,
				seconds: 0,
				furthest: 0,
				verses: verses.reduce((m, v) => Math.max(m, v.numberEnd, v.number), 0)
			};
			this.#seconds = 0;
			this.#savedAt = time;
		}
		if (this.#last !== null && focused) {
			const gap = time - this.#last;
			if (gap > 0 && gap < MAX_GAP) this.#seconds += gap;
		}
		this.#last = time;
		const session = this.#session!;
		session.furthest = Math.max(session.furthest, this.furthest);
		session.seconds = Math.floor(this.#seconds);
		if (time - this.#savedAt > SAVE_EVERY) {
			this.#saveReading();
			this.#savedAt = time;
		}
	}

	#saveReading() {
		const s = this.#session;
		if (s && this.#seconds >= MIN_SECONDS) this.study((c) => c.bible.recordReading(s));
	}

	finishReading() {
		this.#saveReading();
		this.#session = null;
	}

	/** The daily plan reminder: once a day from 7:00, while a plan has today's reading to do (app.rs `plan_reminder`). */
	#remindPlans() {
		if (!this.settings.planReminder || !this.core) return;
		const t = now();
		const today = dayStart(t);
		if (this.#reminded === today || t - today < 7 * 3600) return;
		this.#reminded = today;
		for (const p of this.plans()) {
			if (p.finished !== null) continue;
			const day = Math.floor((today - p.started) / 86_400) + 1;
			const due = planDays(p.id)[day - 1];
			const reads = this.planReads(p.id);
			if (due?.passages.some((_, i) => !reads.has(`${day}:${i}`))) {
				this.toastNow(`Today's reading: ${due.label}`);
				return;
			}
		}
	}

	/** Forgets the chapter being read without saving it, if it began at `since` or later. */
	#dropReading(since = 0) {
		if (this.#session && this.#session.started >= since) this.#session = null;
	}

	// ---------------------------------------------------------------- toasts

	toastNow(text: string, retry?: () => void) {
		const id = ++this.#toastId;
		if (retry) {
			this.toasts = [{ id, text, retry }, ...this.toasts.filter((t) => !t.retry)];
			return;
		}
		this.toasts.push({ id, text });
		setTimeout(() => this.dismissToast(id), TOAST_MS);
	}

	dismissToast(id: number) {
		this.toasts = this.toasts.filter((t) => t.id !== id);
	}

	// ---------------------------------------------------------------- actions

	dispatch = (a: Action): void => {
		const s = this.settings;
		switch (a.type) {
			case 'jump':
				this.page = 'reader';
				this.#jump(a.chapter, a.verse, a.flash ?? false);
				// Landing on a specific verse selects it, so N/B/C/1–6 act on it right away. Anywhere
				// else, a selection left from before would send J/K back to where it was.
				this.setSelection(a.verse && a.flash ? this.refAt(a.chapter, a.verse) : null);
				break;
			case 'back': {
				if (this.page !== 'reader') {
					this.page = 'reader';
					break;
				}
				const to = this.history.back.pop();
				if (to) {
					this.history.forward.push({ ...this.here });
					this.reader?.jump(to.chapter, to.verse, false);
				}
				break;
			}
			case 'forward': {
				this.page = 'reader';
				const to = this.history.forward.pop();
				if (to) {
					this.history.back.push({ ...this.here });
					this.reader?.jump(to.chapter, to.verse, false);
				}
				break;
			}
			case 'prevChapter': {
				const c = this.here.chapter;
				// Within a chapter, ← first returns to its start.
				const target = (this.reader?.offsetInChapter() ?? 0) > 40 ? c : c - 1;
				if (target >= 0) this.reader?.jump(target, undefined, false);
				break;
			}
			case 'nextChapter':
				if (this.here.chapter + 1 < CHAPTERS) this.reader?.jump(this.here.chapter + 1, undefined, false);
				break;
			case 'setTranslation':
				if (this.info(a.id)) {
					s.translation = a.id;
					s.parallel = s.parallel.filter((p) => p !== a.id);
					void this.ensure(a.id);
					this.search.invalidate();
				}
				break;
			case 'translationNumber': {
				const t = this.translations[a.n];
				if (t) this.dispatch({ type: 'setTranslation', id: t.id });
				break;
			}
			case 'toggleParallel':
				s.parallelOn = !s.parallelOn;
				if (s.parallelOn) for (const id of this.columns()) void this.ensure(id);
				break;
			case 'openInParallel':
				this.dispatch(a.then);
				if (!s.parallelOn) this.dispatch({ type: 'toggleParallel' });
				break;
			case 'compareVerse':
				this.setSelection(a.verse);
				this.dispatch({ type: 'compare' });
				break;
			case 'addParallel':
				if (a.id !== s.translation && !s.parallel.includes(a.id)) s.parallel.push(a.id);
				s.parallelOn = true;
				void this.ensure(a.id);
				break;
			case 'replaceParallel': {
				if (a.id === s.translation) break;
				const shown = this.columns().slice(1);
				const at = shown.indexOf(a.id);
				if (at >= 0 && a.index < shown.length) [shown[at], shown[a.index]] = [shown[a.index], shown[at]];
				else if (at < 0 && a.index < shown.length) shown[a.index] = a.id;
				else if (at < 0) shown.push(a.id);
				s.parallel = shown;
				void this.ensure(a.id);
				break;
			}
			case 'removeParallel':
				s.parallel = s.parallel.filter((p) => p !== a.id);
				break;
			case 'highlight': {
				// Chosen words: just those, placed in the text they were chosen in.
				const w = this.wordSpan();
				if (w) {
					this.study((c) => c.bible.highlightSpan(w.translation, w.span, a.color), "Couldn't save highlight");
					break;
				}
				const refs = this.selectedRefs();
				if (refs.length)
					this.study((c) => refs.forEach((r) => c.bible.setHighlight(r, a.color)), "Couldn't save highlight");
				break;
			}
			case 'editNote':
				this.setSelection(a.verse);
				this.note = a.verse;
				break;
			case 'editSelectedNote':
				this.dispatch({ type: 'editNote', verse: this.selectionOrLocation()[0] });
				break;
			case 'toggleBookmark': {
				const v = this.selectionOrLocation()[0];
				const on = this.study((c) => c.bible.toggleBookmark(v), "Couldn't save bookmark");
				if (on !== undefined) this.toastNow(`${on ? 'Bookmarked' : 'Removed bookmark'} ${this.label(v)}`);
				break;
			}
			case 'toggleBookmarkAt':
				this.study((c) => c.bible.toggleBookmark(a.verse));
				break;
			case 'removeBookmark':
				this.study((c) => c.bible.setBookmark(a.verse, null));
				break;
			case 'removeHighlight':
				this.study((c) => c.bible.removeHighlight(a.verse, a.start));
				break;
			case 'removeNote':
				this.study((c) => c.bible.setNote(a.verse, ''));
				if (this.note && refEq(this.note, a.verse)) this.note = null;
				break;
			case 'saveNote':
				this.study((c) => c.bible.setNote(a.verse, a.body), "Couldn't save note");
				break;
			case 'startPlan':
				this.study((c) => c.bible.startPlan(a.id, today()));
				break;
			case 'stopPlan':
				this.study((c) => c.bible.stopPlan(a.id));
				break;
			case 'setPlanRead':
				this.study((c) => c.bible.setPlanRead(a.plan, a.day, a.passage, a.read));
				break;
			case 'finishPlan':
				this.study((c) => c.bible.finishPlan(a.id, a.at));
				break;
			case 'setPlanReminder':
				s.planReminder = a.on;
				break;
			case 'setAccent':
				s.accent = a.accent;
				break;
			case 'setReduceMotion':
				s.reduceMotion = a.on;
				break;
			case 'eraseStudy':
				// A backup first, so it can be undone.
				if (!this.backupNow()) {
					this.toastNow("Couldn't erase: the backup couldn't be saved");
					break;
				}
				if (this.study((c) => (c.bible.eraseStudy(), true), "Couldn't erase")) {
					this.#dropReading();
					this.toastNow('Erased. A backup was saved first (Settings › Data)');
				}
				break;
			case 'toggleFocus':
				this.focus = !this.focus;
				if (this.focus) this.page = 'reader';
				void setFullscreen(this.focus);
				break;
			case 'textFormat':
				this.textPopover = !this.textPopover;
				break;
			case 'clearReading':
				this.study((c) => c.bible.clearReading(a.since));
				this.#dropReading(a.since);
				break;
			case 'copyText': {
				const w = this.wordSpan();
				if (w) {
					const text = this.core?.copySpan(w.translation, w.span, 'plain');
					if (text) void this.#copy(text, 'Copied');
					break;
				}
				const refs = this.selectedRefs();
				const text = this.core?.copyText(s.translation, refs, 'plain');
				if (refs.length && text) void this.#copy(text, 'Copied');
				break;
			}
			case 'copy': {
				const w = this.wordSpan();
				if (w) {
					const text = this.core?.copySpan(w.translation, w.span, 'reference');
					if (text) void this.#copy(text, 'Copied with reference');
					break;
				}
				const refs = this.selectedRefs();
				const text = this.core?.copyText(s.translation, refs, 'reference');
				if (refs.length && text) void this.#copy(text, 'Copied with reference');
				break;
			}
			case 'openVerseMenu':
				if (this.selectedRefs().length) this.verseMenu = { x: a.x, y: a.y };
				break;
			case 'shareImage': {
				const refs = this.selectedRefs();
				if (refs.length) this.share = refs;
				break;
			}
			case 'compare': {
				const v = this.selectionOrLocation()[0];
				this.setSelection(v);
				if (this.columns().length > 1) this.panel = v;
				else this.compare = v;
				for (const t of this.translations) void this.ensure(t.id);
				break;
			}
			case 'searchStep':
				this.search.step(a.delta);
				break;
			case 'clearSearch':
				this.search.clear();
				this.selection = null;
				break;
			case 'clearSelection':
				if (this.search.searching) this.dispatch({ type: 'clearSearch' });
				else if (this.selection) this.selection = null;
				else this.panel = null;
				break;
			case 'selectNext':
				this.#stepSelection(1);
				break;
			case 'selectPrev':
				this.#stepSelection(-1);
				break;
			case 'openPalette':
				this.palette = true;
				break;
			case 'openSearch':
				s.sidebarOpen = true;
				s.sidebarTab = 'search';
				this.search.focus(a.query);
				break;
			case 'toggleSidebar':
				s.sidebarOpen = !s.sidebarOpen;
				break;
			case 'sidebarTab':
				s.sidebarOpen = true;
				s.sidebarTab = a.tab;
				break;
			case 'openSettings':
				this.settingsTab = a.tab;
				break;
			case 'fontSize':
				s.typo.size = Math.min(34, Math.max(12, s.typo.size + a.delta));
				break;
			case 'fontReset':
				s.typo.size = defaultTypography().size;
				break;
			case 'setTheme':
				s.theme = a.theme;
				s.themeSystem = false;
				break;
			case 'cycleTheme':
				this.dispatch({ type: 'setTheme', theme: THEMES[(THEMES.indexOf(s.theme) + 1) % THEMES.length] });
				break;
			case 'toggleMode':
				s.typo.mode = s.typo.mode === 'paragraph' ? 'versePerLine' : 'paragraph';
				break;
			case 'toggleRedLetters':
				s.typo.redLetters = !s.typo.redLetters;
				break;
			case 'export':
				this.exportStudy(a.kind);
				break;
			case 'restoreBackup': {
				const last = this.backups()[0];
				if (last) this.importStudyText(last.json);
				break;
			}
			case 'importStudy':
				void pickFile('.json,application/json').then((f) => f?.text().then((t) => this.importStudyText(t)));
				break;
			case 'importTranslation':
				this.importSheet = true;
				if (a.folder && importer.stage === 'choose') void importer.pick(true);
				break;
			case 'removeTranslation':
				void this.removeImported(a.id);
				break;
			case 'openPage':
				if (this.page !== a.page) this.pageQuery = '';
				this.page = a.page;
				break;
			case 'showShortcuts':
				this.shortcuts = !this.shortcuts;
				break;
			case 'scrollLines':
				this.reader?.scrollBy(a.n * s.typo.size * s.typo.lineHeight);
				break;
			case 'scrollPages':
				this.reader?.scrollBy(a.n * (this.reader?.page() ?? 0));
				break;
			case 'toast':
				this.toastNow(a.text);
				break;
		}
	};

	/** "John 3:16". */
	label(v: VerseRef): string {
		return `${chapterLabel(this.chapterIndex(v))}:${v.verse}`;
	}

	/** The number of verses in a chapter (the canonical count). */
	chapterVerses(index: number): number {
		return chapterVerses(index);
	}

	async #copy(text: string, done: string) {
		try {
			await navigator.clipboard.writeText(text);
			this.toastNow(done);
		} catch {
			this.toastNow("Couldn't copy");
		}
	}

	saveSettingsNow() {
		this.settings.chapter = this.here.chapter;
		this.settings.verse = this.here.verse;
		saveSettings($state.snapshot(this.settings));
	}
}

function download(name: string, body: string, type: string) {
	const url = URL.createObjectURL(new Blob([body], { type }));
	const a = document.createElement('a');
	a.href = url;
	a.download = name;
	a.click();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Opens the browser's file picker; resolves with the file, or null when cancelled. */
export function pickFile(accept: string): Promise<File | null> {
	return new Promise((resolve) => {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = accept;
		input.onchange = () => resolve(input.files?.[0] ?? null);
		input.oncancel = () => resolve(null);
		input.click();
	});
}

async function setFullscreen(on: boolean) {
	try {
		if (on && !document.fullscreenElement) await document.documentElement.requestFullscreen?.();
		if (!on && document.fullscreenElement) await document.exitFullscreen?.();
	} catch {
		// Fullscreen refused (iOS Safari, iframes): focus mode still hides the chrome.
	}
}

const KEY = Symbol('read');
export const setApp = (app: ReadApp) => setContext(KEY, app);
export const getApp = () => getContext<ReadApp>(KEY);
