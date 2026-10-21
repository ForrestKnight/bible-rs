/**
 * The reader's settings: the Mac app's `Settings` (crates/app/src/app.rs), kept in localStorage.
 * Missing or unreadable fields fall back to the defaults, so older saves keep working.
 */
import { DEFAULT_CHAPTER, DEFAULT_TRANSLATION, type ContentOptions } from './core';

export type Theme = 'light' | 'sepia' | 'dark';
export type Accent = 'ink' | 'oxblood' | 'forest' | 'plum' | 'graphite';
export type SidebarTab = 'books' | 'search' | 'study';
export type Mode = 'paragraph' | 'versePerLine';
export type CardStyle = 'paper' | 'sepia' | 'night' | 'ink';
export type CardShape = 'square' | 'portrait' | 'wide';
export type ShareCopy = 'plain' | 'reference' | 'markdown';

export const ACCENTS: Accent[] = ['ink', 'oxblood', 'forest', 'plum', 'graphite'];
export const THEMES: Theme[] = ['light', 'sepia', 'dark'];

export interface Typography {
	/** Reading size in px (the Mac's points). */
	size: number;
	/** Line height as a multiple of the size. */
	lineHeight: number;
	mode: Mode;
	redLetters: boolean;
	justify: boolean;
	verseNumbers: boolean;
	headings: boolean;
	/** Maximum measure in ems. */
	measure: number;
}

export interface ShareOptions {
	style: CardStyle;
	shape: CardShape;
	reference: boolean;
	translation: boolean;
	redLetters: boolean;
	copy: ShareCopy;
}

export interface Settings {
	translation: string;
	parallel: string[];
	parallelOn: boolean;
	theme: Theme;
	/** Follow the system's light or dark appearance instead of `theme`. */
	themeSystem: boolean;
	typo: Typography;
	sidebarOpen: boolean;
	sidebarTab: SidebarTab;
	chapter: number;
	verse: number;
	wholeWords: boolean;
	accent: Accent;
	toolbarTranslation: boolean;
	sidebarAttached: boolean;
	/** Names for the highlight colours, in HIGHLIGHT_COLORS order; empty for the colour's own. */
	highlightNames: string[];
	reduceMotion: boolean;
	flashJumps: boolean;
	exportInclude: { highlights: boolean; notes: boolean; bookmarks: boolean; history: boolean };
	/** Automatic backups (kept in the browser), weekly rather than daily. */
	backupAuto: boolean;
	backupWeekly: boolean;
	/** Chords chosen in Settings › Shortcuts, by row id. */
	shortcuts: Record<string, string>;
	share: ShareOptions;
	planReminder: boolean;
	onboarded: boolean;
}

export const defaultTypography = (): Typography => ({
	size: 20,
	lineHeight: 1.6,
	mode: 'paragraph',
	redLetters: true,
	justify: false,
	verseNumbers: true,
	headings: true,
	measure: 640 / 20
});

export const defaultSettings = (): Settings => ({
	translation: DEFAULT_TRANSLATION,
	parallel: ['web'],
	parallelOn: false,
	theme: 'light',
	themeSystem: false,
	typo: defaultTypography(),
	sidebarOpen: false,
	sidebarTab: 'books',
	chapter: DEFAULT_CHAPTER,
	verse: 1,
	wholeWords: false,
	accent: 'ink',
	toolbarTranslation: true,
	sidebarAttached: false,
	highlightNames: [],
	reduceMotion: false,
	flashJumps: true,
	exportInclude: { highlights: true, notes: true, bookmarks: true, history: false },
	backupAuto: true,
	backupWeekly: false,
	shortcuts: {},
	share: {
		style: 'sepia',
		shape: 'portrait',
		reference: true,
		translation: true,
		redLetters: false,
		copy: 'reference'
	},
	planReminder: false,
	onboarded: false
});

/** What the core needs of the typography to break a chapter into blocks. */
export const contentOptions = (t: Typography): ContentOptions => ({
	versePerLine: t.mode === 'versePerLine',
	headings: t.headings,
	verseNumbers: t.verseNumbers
});

export const SETTINGS_KEY = 'bible.read.settings';
export const STUDY_KEY = 'bible.read.study';

/** Fills what a saved value lacks (or has the wrong type for) from `fallback`, recursively. */
export function merge<T>(fallback: T, saved: unknown): T {
	if (saved === undefined || saved === null) return fallback;
	if (Array.isArray(fallback)) return (Array.isArray(saved) ? saved : fallback) as T;
	if (typeof fallback === 'object' && fallback !== null) {
		if (typeof saved !== 'object' || Array.isArray(saved)) return fallback;
		const out: Record<string, unknown> = { ...(saved as object) };
		for (const [k, v] of Object.entries(fallback)) {
			out[k] = merge(v, (saved as Record<string, unknown>)[k]);
		}
		return out as T;
	}
	return (typeof saved === typeof fallback ? saved : fallback) as T;
}

export function loadSettings(): Settings {
	try {
		const raw = localStorage.getItem(SETTINGS_KEY);
		if (raw) return merge(defaultSettings(), JSON.parse(raw));
	} catch {
		// Unreadable or blocked storage: start fresh.
	}
	return defaultSettings();
}

export function saveSettings(s: Settings) {
	try {
		localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
	} catch {
		// Full or blocked storage: settings last for the visit.
	}
}
