/**
 * Keyboard shortcuts as the Mac app has them (crates/app/src/keymap.rs and `BibleApp::shortcuts`):
 * three groups of rows, the changeable ones with chords chosen in Settings › Shortcuts. ⌘ is the
 * Command key on a Mac and Control elsewhere (where ⌃⌘ becomes Control-Alt).
 */
import type { Action, ReadApp } from './app.svelte';
import { HIGHLIGHT_COLORS } from './core';

export interface Chord {
	ctrl: boolean;
	alt: boolean;
	shift: boolean;
	cmd: boolean;
	/** "K", "1", "ArrowLeft", "Space", "[", … */
	key: string;
}

export interface Shortcut {
	id: string;
	group: 'Navigate' | 'Verses' | 'View';
	label: string;
	/** A changeable row's default chord; null for a fixed (paired) row. */
	chord: Chord | null;
	/** A fixed row's keycaps. */
	fixed: string[];
}

export const isMac =
	typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);

const chord = (key: string, m: Partial<Chord> = {}): Chord => ({
	ctrl: false,
	alt: false,
	shift: false,
	cmd: false,
	key,
	...m
});
const bindable = (id: string, group: Shortcut['group'], label: string, c: Chord): Shortcut => ({
	id,
	group,
	label,
	chord: c,
	fixed: []
});
const fixed = (id: string, group: Shortcut['group'], label: string, caps: string[]): Shortcut => ({
	id,
	group,
	label,
	chord: null,
	fixed: caps
});

export const GROUPS = ['Navigate', 'Verses', 'View'] as const;

export const SHORTCUTS: Shortcut[] = [
	bindable('jump', 'Navigate', 'Jump to a reference', chord('K', { cmd: true })),
	fixed('chapter', 'Navigate', 'Previous / next chapter', ['←', '→']),
	fixed('history', 'Navigate', 'Back / forward', ['⌘', '[', '⌘', ']']),
	bindable('search', 'Navigate', 'Search', chord('F', { cmd: true })),
	bindable('sidebar', 'Navigate', 'Books & study sidebar', chord('B', { cmd: true })),
	fixed('verse', 'Verses', 'Next / previous verse', ['J', 'K']),
	fixed('highlight', 'Verses', 'Highlight colour · clear', ['1–6', '0']),
	bindable('note', 'Verses', 'Note', chord('N')),
	bindable('bookmark', 'Verses', 'Bookmark', chord('B')),
	bindable('copy', 'Verses', 'Copy with reference', chord('C', { shift: true, cmd: true })),
	bindable('compare', 'Verses', 'Compare translations', chord('C')),
	fixed('translation', 'View', 'Switch translation', ['⌘', '1…9']),
	bindable('parallel', 'View', 'Parallel translations', chord('P', { shift: true, cmd: true })),
	fixed('size', 'View', 'Text size', ['⌘', '+', '⌘', '−']),
	bindable('mode', 'View', 'Paragraphs / verse per line', chord('L', { cmd: true })),
	bindable('red', 'View', 'Red letters', chord('R', { cmd: true })),
	bindable('theme', 'View', 'Theme', chord('L', { shift: true, cmd: true })),
	bindable('focus', 'View', 'Focus mode', chord('F', { ctrl: true, cmd: true }))
];

/** What a changeable row does. */
export function rowAction(id: string): Action | null {
	switch (id) {
		case 'jump':
			return { type: 'openPalette' };
		case 'search':
			return { type: 'openSearch' };
		case 'sidebar':
			return { type: 'toggleSidebar' };
		case 'note':
			return { type: 'editSelectedNote' };
		case 'bookmark':
			return { type: 'toggleBookmark' };
		case 'copy':
			return { type: 'copy' };
		case 'compare':
			return { type: 'compare' };
		case 'parallel':
			return { type: 'toggleParallel' };
		case 'mode':
			return { type: 'toggleMode' };
		case 'red':
			return { type: 'toggleRedLetters' };
		case 'theme':
			return { type: 'cycleTheme' };
		case 'focus':
			return { type: 'toggleFocus' };
	}
	return null;
}

/** "ctrl+cmd+F", as stored in the settings (the Mac's spelling). */
export const spec = (c: Chord) =>
	[c.ctrl && 'ctrl', c.alt && 'alt', c.shift && 'shift', c.cmd && 'cmd', c.key].filter(Boolean).join('+');

export function parse(s: string): Chord | null {
	const c = chord('');
	for (const part of s.split('+')) {
		if (part === 'ctrl') c.ctrl = true;
		else if (part === 'alt') c.alt = true;
		else if (part === 'shift') c.shift = true;
		else if (part === 'cmd') c.cmd = true;
		else c.key = part;
	}
	return c.key ? c : null;
}

const GLYPHS: Record<string, string> = {
	ArrowLeft: '←',
	ArrowRight: '→',
	ArrowUp: '↑',
	ArrowDown: '↓',
	'+': '+',
	'=': '+',
	'-': '−',
	Space: 'Space'
};

/** Keycaps in the macOS order: ⌃ ⌥ ⇧ ⌘, then the key; Ctrl/Alt/Shift words elsewhere. */
export function keycaps(c: Chord): string[] {
	const caps: string[] = [];
	if (isMac) {
		if (c.ctrl) caps.push('⌃');
		if (c.alt) caps.push('⌥');
		if (c.shift) caps.push('⇧');
		if (c.cmd) caps.push('⌘');
	} else {
		if (c.cmd || c.ctrl) caps.push('Ctrl');
		if (c.alt || (c.ctrl && c.cmd)) caps.push('Alt');
		if (c.shift) caps.push('Shift');
	}
	caps.push(GLYPHS[c.key] ?? c.key.toUpperCase());
	return caps;
}

/** A fixed row's caps, with ⌘ spelled for this platform. */
export const fixedCaps = (caps: string[]) => (isMac ? caps : caps.map((c) => (c === '⌘' ? 'Ctrl' : c)));

/** A row's chord: its override (empty when it was taken by another row), else its default. */
export const chordFor = (s: Shortcut, overrides: Record<string, string>): Chord | null =>
	s.id in overrides ? parse(overrides[s.id]) : s.chord;

export const rowCaps = (s: Shortcut, overrides: Record<string, string>) => {
	const c = chordFor(s, overrides);
	return s.chord ? (c ? keycaps(c) : []) : fixedCaps(s.fixed);
};

/** Keycaps as one string: "⇧⌘P" on a Mac, "Ctrl+Shift+P" elsewhere. */
export const capsText = (caps: string[]) => caps.join(isMac ? '' : '+');

/** A changeable row's chord as text, as currently bound ("" when unbound). */
export function shortcutText(id: string, overrides: Record<string, string>): string {
	const s = SHORTCUTS.find((s) => s.id === id);
	return s ? capsText(rowCaps(s, overrides)) : '';
}

/** A fixed chord as text: `cmd('T')` is "⌘T" on a Mac, "Ctrl+T" elsewhere. */
export const cmdText = (key: string) => capsText(fixedCaps(['⌘', key]));

/** The key a keyboard event names, as chords spell it. */
export function eventKey(e: KeyboardEvent): string {
	if (e.code === 'Space') return 'Space';
	if (/^Key[A-Z]$/.test(e.code)) return e.code.slice(3);
	if (/^Digit\d$/.test(e.code)) return e.code.slice(5);
	if (e.code === 'BracketLeft') return '[';
	if (e.code === 'BracketRight') return ']';
	if (e.code === 'Equal') return '=';
	if (e.code === 'Minus') return '-';
	if (e.code === 'Slash') return '/';
	if (e.code === 'Comma') return ',';
	if (e.code === 'Period') return '.';
	return e.key.length === 1 ? e.key.toUpperCase() : e.key;
}

/** A chord from a key press, for Settings › Shortcuts; null for keys a shortcut can't use. */
export function fromEvent(e: KeyboardEvent): Chord | null {
	if (['Escape', 'Tab', 'Enter', 'Backspace', 'Delete', 'Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return null;
	const key = eventKey(e);
	if (isMac) return { ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey, cmd: e.metaKey, key };
	return { ctrl: e.ctrlKey && e.altKey, alt: e.altKey && !e.ctrlKey, shift: e.shiftKey, cmd: e.ctrlKey, key };
}

/** The modifier keys a chord means on this platform. */
function held(c: Chord) {
	if (isMac) return { meta: c.cmd, ctrl: c.ctrl, alt: c.alt, shift: c.shift };
	return { meta: false, ctrl: c.cmd || c.ctrl, alt: c.alt || (c.cmd && c.ctrl), shift: c.shift };
}

function matches(e: KeyboardEvent, c: Chord): boolean {
	const h = held(c);
	return (
		e.metaKey === h.meta &&
		e.ctrlKey === h.ctrl &&
		e.altKey === h.alt &&
		e.shiftKey === h.shift &&
		eventKey(e) === c.key
	);
}

const modifierCount = (c: Chord) => [c.ctrl, c.alt, c.shift, c.cmd].filter(Boolean).length;

/** Whether the keyboard is in a text field. */
function typing(): boolean {
	const el = document.activeElement as HTMLElement | null;
	return !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));
}

/**
 * Whether Space should press the focused control rather than scroll: one in a menu or popover, or
 * one reached from the keyboard (a button clicked keeps focus, and Space still scrolls then).
 */
function pressable(): boolean {
	const el = document.activeElement as HTMLElement | null;
	const control =
		'button, a[href], [role^="menuitem"], [role="button"], [role="checkbox"], [role="radio"], [role="switch"], [role="tab"]';
	return !!el?.matches(control) && (!!el.closest('.popover') || el.matches(':focus-visible'));
}

/** The reader's shortcuts; returns whether the key was taken. */
export function handleKey(app: ReadApp, e: KeyboardEvent): boolean {
	if (e.defaultPrevented || e.isComposing || app.onboarding) return false;
	const go = (a: Action) => {
		e.preventDefault();
		app.dispatch(a);
		return true;
	};
	const overrides = app.settings.shortcuts;
	const bindings = SHORTCUTS.flatMap((s) => {
		const c = chordFor(s, overrides);
		const a = rowAction(s.id);
		return c && a ? [{ c, a }] : [];
	}).sort((x, y) => modifierCount(y.c) - modifierCount(x.c));
	const anywhere = (a: Action) => a.type === 'openPalette' || a.type === 'openSearch';
	const isCommand = (c: Chord) => c.cmd || c.ctrl;
	const key = eventKey(e);
	const cmd = isMac ? e.metaKey : e.ctrlKey;
	const plain = !e.metaKey && !e.ctrlKey && !e.altKey;
	const only = (k: string, m: Partial<Chord> = {}) => matches(e, chord(k, m));

	for (const { c, a } of bindings) if (anywhere(a) && matches(e, c)) return go(a);
	if (only(',', { cmd: true })) return go({ type: 'openSettings', tab: 'reading' });
	if (only('/', { cmd: true })) return go({ type: 'showShortcuts' });
	if (app.palette || app.settingsTab || app.shortcuts || app.share || app.importSheet) return false;

	for (const { c, a } of bindings) {
		if (isCommand(c) && !anywhere(a) && a.type !== 'copy' && matches(e, c)) return go(a);
	}
	if (only('E', { cmd: true, shift: true })) return go({ type: 'export', kind: 'json' });
	if (only('[', { cmd: true }) || only('ArrowLeft', { alt: true })) return go({ type: 'back' });
	if (only(']', { cmd: true }) || only('ArrowRight', { alt: true })) return go({ type: 'forward' });
	if (only('=', { cmd: true }) || only('=', { cmd: true, shift: true }) || only('+', { cmd: true }))
		return go({ type: 'fontSize', delta: 1 });
	if (only('-', { cmd: true })) return go({ type: 'fontSize', delta: -1 });
	if (only('0', { cmd: true })) return go({ type: 'fontReset' });
	if (only('T', { cmd: true })) return go({ type: 'textFormat' });
	if (only('D', { cmd: true })) return go({ type: 'toggleBookmark' });
	if (only('E', { cmd: true })) return go({ type: 'export', kind: 'markdown' });
	if (only('I', { cmd: true })) return go({ type: 'importTranslation' });
	if (cmd && !e.shiftKey && !e.altKey && /^[1-9]$/.test(key)) return go({ type: 'translationNumber', n: Number(key) - 1 });
	if (typing()) return false;

	for (const { c, a } of bindings) {
		if ((a.type === 'copy' || !isCommand(c)) && matches(e, c)) return go(a);
	}
	// ⌘C copies the selected verses, unless text is selected in the page.
	if (only('C', { cmd: true }) && getSelection()?.isCollapsed !== false) {
		if (!app.selectedRefs().length) return false;
		return go({ type: 'copyText' });
	}
	if (!plain) return false;
	const shift = e.shiftKey;
	if (!shift && key === 'ArrowLeft') return go({ type: 'prevChapter' });
	if (!shift && key === 'ArrowRight') return go({ type: 'nextChapter' });
	if (!shift && key === 'ArrowDown') return go({ type: 'scrollLines', n: 3 });
	if (!shift && key === 'ArrowUp') return go({ type: 'scrollLines', n: -3 });
	if (key === 'PageDown') return go({ type: 'scrollPages', n: 1 });
	if (key === 'PageUp') return go({ type: 'scrollPages', n: -1 });
	if (key === 'Space') return pressable() ? false : go({ type: 'scrollPages', n: shift ? -1 : 1 });
	if (!shift && key === 'J') return go({ type: 'selectNext' });
	if (!shift && key === 'K') return go({ type: 'selectPrev' });
	if (key === 'Escape') return go({ type: 'clearSelection' });
	if (!shift && key === '/') return go({ type: 'openSearch' });
	if (e.key === '?') return go({ type: 'showShortcuts' });
	if (!shift && /^[1-6]$/.test(key)) return go({ type: 'highlight', color: HIGHLIGHT_COLORS[Number(key) - 1] });
	if (!shift && (key === '0' || key === 'Backspace')) return go({ type: 'highlight', color: null });
	return false;
}
