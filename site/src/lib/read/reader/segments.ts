import type { Block, HighlightColor, HighlightRange, Run } from '../core';

/**
 * A verse's part of a block: its inline number, its text (what a highlight or the selection
 * washes) and the space after it, which the wash leaves out, as on the Mac (reader/mod.rs).
 */
export interface Segment {
	verse: number;
	number: Run | null;
	runs: Run[];
	trail: string;
	/** The verse begins here (its margin markers and its place for scrolling go here). */
	start: boolean;
}

/** Splits each block into segments; `seen` carries the verses already begun across blocks. */
export function segments(block: Block, seen: Set<number>): Segment[] {
	const out: Segment[] = [];
	let cur: Segment | null = null;
	for (const run of block.runs) {
		if (run.number || !cur || cur.verse !== run.verse) {
			cur = { verse: run.verse, number: run.number ? run : null, runs: [], trail: '', start: false };
			out.push(cur);
			if (run.number) continue;
		}
		cur.runs.push(run);
	}
	for (const seg of out) {
		const last = seg.runs[seg.runs.length - 1];
		const trimmed = last?.text.replace(/\s+$/, '');
		if (last && trimmed !== last.text) {
			seg.trail = last.text.slice(trimmed!.length);
			seg.runs[seg.runs.length - 1] = { ...last, text: trimmed! };
		}
		if (seg.verse > 0 && !seen.has(seg.verse)) {
			seen.add(seg.verse);
			seg.start = true;
		}
	}
	return out;
}

/** A stretch of a run's text, with where it starts in its verse's text (`data-o`, which maps a
 *  text selection back to the verse). A space the layout added has no place of its own there. */
export interface Piece {
	text: string;
	offset: number;
	cls: string;
	added: boolean;
}

/** Neighbouring pieces in one highlight colour, or in none. */
export interface Wash {
	color: HighlightColor | null;
	pieces: Piece[];
}

/** A verse's runs cut where its highlights start and end, gathered by colour. */
export function washes(runs: Run[], ranges: HighlightRange[], redLetters: boolean): Wash[] {
	const out: Wash[] = [];
	const push = (piece: Piece, color: HighlightColor | null) => {
		const last = out[out.length - 1];
		if (last && last.color === color) last.pieces.push(piece);
		else out.push({ color, pieces: [piece] });
	};
	for (const r of runs) {
		if (!r.text) continue;
		const cls = runClass(r, redLetters);
		if (r.added) {
			// Between two chars: coloured only inside a highlight that goes on past it.
			const color = ranges.find((h) => h.start < r.offset && r.offset < h.end)?.color ?? null;
			push({ text: r.text, offset: r.offset, cls, added: true }, color);
			continue;
		}
		const end = r.offset + r.text.length;
		const cuts = new Set([r.offset, end]);
		for (const h of ranges) for (const x of [h.start, h.end]) if (x > r.offset && x < end) cuts.add(x);
		const points = [...cuts].sort((a, b) => a - b);
		for (let i = 0; i + 1 < points.length; i++) {
			const [a, b] = [points[i], points[i + 1]];
			const color = ranges.find((h) => h.start <= a && a < h.end)?.color ?? null;
			push({ text: r.text.slice(a - r.offset, b - r.offset), offset: a, cls, added: false }, color);
		}
	}
	return out;
}

/** A run's classes: italic, semibold, small capitals, words of Christ. */
export function runClass(r: Run, redLetters: boolean): string {
	let c = '';
	if (r.italic) c += ' i';
	if (r.bold) c += ' b';
	if (r.smallCaps) c += ' sc';
	if (r.red && redLetters) c += ' red';
	return c.trim();
}
