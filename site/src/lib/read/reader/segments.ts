import type { Block, Run } from '../core';

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

/** A run's classes: italic, semibold, small capitals, words of Christ. */
export function runClass(r: Run, redLetters: boolean): string {
	let c = '';
	if (r.italic) c += ' i';
	if (r.bold) c += ' b';
	if (r.smallCaps) c += ' sc';
	if (r.red && redLetters) c += ' red';
	return c.trim();
}
