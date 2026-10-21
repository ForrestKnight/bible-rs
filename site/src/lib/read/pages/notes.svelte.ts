/**
 * Notes as Markdown (pages/notes.rs): the plain text for the list, the verses a note mentions, the
 * format bar's edits, and the lines styled for reading.
 */
import { parseReference, type VerseRef } from '../core';

/** Kept across visits to the page, as the Mac keeps it. */
export const notesPage = $state<{ recent: boolean; selected: VerseRef | null }>({ recent: true, selected: null });

export const sameVerse = (a: VerseRef, b: VerseRef) => a.book === b.book && a.chapter === b.chapter && a.verse === b.verse;

/** The note as plain text for the list: markers dropped, lines joined. */
export const plain = (md: string) =>
	md
		.split('\n')
		.map((l) => l.replace(/^[#>\-* ]+/, '').replaceAll('**', '').replaceAll('*', ''))
		.filter((l) => l.trim())
		.join(' ');

/** Verse references a note mentions ("Isaiah 9:2", "1 John 2:8"), in order, without repeats. */
export function mentioned(text: string): VerseRef[] {
	const words = text.split(/[\s()]/);
	const out: VerseRef[] = [];
	words.forEach((w, i) => {
		const cv = w.replace(/[,.;:—–!?”"]+$/, '');
		const colon = cv.indexOf(':');
		if (colon < 1 || !/^\d+$/.test(cv.slice(0, colon)) || !/^\d/.test(cv.slice(colon + 1))) return;
		// The book: up to three words before ("Song of Songs", "1 John").
		for (let n = 3; n >= 1; n--) {
			if (i < n) continue;
			const r = parseReference(`${words.slice(i - n, i).join(' ')} ${cv}`);
			if (r?.verse) {
				const v = { book: r.book, chapter: r.chapter, verse: r.verse };
				if (!out.some((o) => sameVerse(o, v))) out.push(v);
				break;
			}
		}
	});
	return out;
}

export type Tool = 'heading' | 'bold' | 'italic' | 'list' | 'quote' | 'link';

interface Span {
	start: number;
	end: number;
	bold: boolean;
	/** Where its markers are. */
	open: [number, number];
	close: [number, number];
}

/** `**bold**` and `*italic*` spans in a line; a marker without a partner stays literal. */
function spans(line: string): Span[] {
	const out: Span[] = [];
	let i = 0;
	while (i < line.length) {
		if (line[i] === '*') {
			const bold = line[i + 1] === '*';
			const m = bold ? 2 : 1;
			const open = i + m;
			const c = line.indexOf(bold ? '**' : '*', open);
			if (c > open) {
				out.push({ start: open, end: c, bold, open: [i, open], close: [c, c + m] });
				i = c + m;
				continue;
			}
		}
		i++;
	}
	return out;
}

const PREFIXES = ['# ', '- ', '> '];

/** The styles in effect at `c` (the format bar's on states). */
export function stylesAt(text: string, c: number): Record<Exclude<Tool, 'link'>, boolean> {
	const start = text.lastIndexOf('\n', c - 1) + 1;
	const endAt = text.indexOf('\n', start);
	const line = text.slice(start, endAt < 0 ? text.length : endAt);
	const col = c - start;
	const on = { heading: /^#{1,3} /.test(line), list: line.startsWith('- '), quote: line.startsWith('> '), bold: false, italic: false };
	for (const s of spans(line)) {
		if (s.start <= col && col <= s.end) {
			if (s.bold) on.bold = true;
			else on.italic = true;
		}
	}
	return on;
}

/** Applies a format to the selection `a..b`; returns the text and the new selection. */
export function apply(tool: Tool, text: string, a: number, b: number): [string, number, number] {
	const start = text.lastIndexOf('\n', a - 1) + 1;
	if (tool === 'heading' || tool === 'list' || tool === 'quote') {
		const p = tool === 'heading' ? '# ' : tool === 'list' ? '- ' : '> ';
		const endAt = text.indexOf('\n', start);
		const end = endAt < 0 ? text.length : endAt;
		const line = text.slice(start, end);
		let next: string;
		let delta: number;
		if (line.startsWith(p)) {
			next = line.slice(p.length);
			delta = -p.length;
		} else {
			// One block style per line: drop another prefix first.
			const other = PREFIXES.find((q) => line.startsWith(q));
			next = p + (other ? line.slice(other.length) : line);
			delta = p.length - (other?.length ?? 0);
		}
		const shift = (x: number) => Math.max(start, x + delta);
		return [text.slice(0, start) + next + text.slice(end), shift(a), shift(b)];
	}
	if (tool === 'bold' || tool === 'italic') {
		const m = tool === 'bold' ? '**' : '*';
		const before = text.slice(0, a);
		const after = text.slice(b);
		if (before.endsWith(m) && after.startsWith(m))
			return [before.slice(0, -m.length) + text.slice(a, b) + after.slice(m.length), a - m.length, b - m.length];
		return [before + m + text.slice(a, b) + m + after, a + m.length, b + m.length];
	}
	return [text, a, b];
}

export interface Run {
	text: string;
	bold?: boolean;
	italic?: boolean;
}

export interface Line {
	kind: 'p' | 'heading' | 'bullet' | 'quote' | 'gap';
	runs: Run[];
}

/** The note's lines styled: headings, bullets, quotes, bold and italic; a blank line a gap. */
export function lines(md: string): Line[] {
	return md.split('\n').map((line) => {
		if (!line) return { kind: 'gap', runs: [] };
		let kind: Line['kind'] = 'p';
		let rest = line;
		const h = /^#{1,3} /.exec(line);
		if (h) [kind, rest] = ['heading', line.slice(h[0].length)];
		else if (/^[-*] /.test(line)) [kind, rest] = ['bullet', line.slice(2)];
		else if (line.startsWith('> ')) [kind, rest] = ['quote', line.slice(2)];
		const runs: Run[] = [];
		let at = 0;
		for (const s of spans(rest)) {
			if (s.open[0] > at) runs.push({ text: rest.slice(at, s.open[0]) });
			runs.push({ text: rest.slice(s.start, s.end), bold: s.bold, italic: !s.bold });
			at = s.close[1];
		}
		if (at < rest.length) runs.push({ text: rest.slice(at) });
		return { kind, runs };
	});
}
