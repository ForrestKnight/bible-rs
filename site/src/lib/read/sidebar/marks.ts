import { HIGHLIGHT_COLORS, type HighlightColor, type StudyItem } from '../core';

/** A marked verse: its highlight colour, or `null` for a note on a verse with no highlight. */
export type Mark = HighlightColor | null;

/** A book's marked verses, and how many highlights and notes it holds. */
export interface BookMarks {
	marks: Mark[];
	highlights: number;
	notes: number;
}

export interface Activity {
	/** By chapter index. */
	chapters: Map<number, Mark[]>;
	/** By book index. */
	books: Map<number, BookMarks>;
}

const rank = (m: Mark) => (m === null ? HIGHLIGHT_COLORS.length : HIGHLIGHT_COLORS.indexOf(m));

/** Every chapter and book with highlights or notes, and its marks: one per verse (a highlighted
 *  verse with a note counts once), sorted by colour so like colours sit together on the ring. */
export function activity(highlights: StudyItem[], notes: StudyItem[]): Activity {
	const verses = new Map<string, { item: StudyItem; mark: Mark }>();
	for (const h of highlights) verses.set(key(h), { item: h, mark: h.highlight });
	for (const n of notes) if (!verses.has(key(n))) verses.set(key(n), { item: n, mark: n.highlight });
	const out: Activity = { chapters: new Map(), books: new Map() };
	const book = (b: number) => {
		let m = out.books.get(b);
		if (!m) out.books.set(b, (m = { marks: [], highlights: 0, notes: 0 }));
		return m;
	};
	for (const { item, mark } of verses.values()) {
		const chapter = out.chapters.get(item.chapterIndex) ?? [];
		chapter.push(mark);
		out.chapters.set(item.chapterIndex, chapter);
		const b = book(item.verse.book);
		b.marks.push(mark);
		if (mark !== null) b.highlights++;
	}
	for (const n of notes) book(n.verse.book).notes++;
	for (const m of out.chapters.values()) m.sort((a, b) => rank(a) - rank(b));
	for (const b of out.books.values()) b.marks.sort((x, y) => rank(x) - rank(y));
	return out;
}

function key(i: StudyItem): string {
	return `${i.verse.book}:${i.verse.chapter}:${i.verse.verse}`;
}

/** "3 highlights · 1 note". */
export function summary(b: BookMarks): string {
	const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`;
	if (!b.notes) return plural(b.highlights, 'highlight');
	if (!b.highlights) return plural(b.notes, 'note');
	return `${plural(b.highlights, 'highlight')} · ${plural(b.notes, 'note')}`;
}

/** The ring's segments as [mark, weight]: one per marked verse, or past `cap` one per colour,
 *  weighted by its count. `marks` must be sorted by colour. */
export function ringSegments(marks: Mark[], cap: number): [Mark, number][] {
	if (marks.length <= cap) return marks.map((m) => [m, 1]);
	const out: [Mark, number][] = [];
	for (const m of marks) {
		const last = out.at(-1);
		if (last && last[0] === m) last[1]++;
		else out.push([m, 1]);
	}
	return out;
}

/** The colour a chapter's number is tinted with: its most used highlight colour (ties go to the
 *  earlier colour), or `null` when it only has notes. */
export function dominantColor(marks: Mark[]): HighlightColor | null {
	let best: HighlightColor | null = null;
	let bestN = 0;
	for (const c of HIGHLIGHT_COLORS) {
		const n = marks.filter((m) => m === c).length;
		if (n > bestN) [best, bestN] = [c, n];
	}
	return best;
}

export const swatch = (m: Mark) => (m === null ? 'var(--accent)' : `var(--swatch-${m})`);
export const tint = (c: HighlightColor | null) => (c === null ? 'var(--accent-soft)' : `var(--hl-${c})`);

export interface Arc {
	mark: Mark;
	/** An SVG path, or null for a whole circle. */
	d: string | null;
}

/** The ring's arcs, clockwise from the top, centred at (`c`, `c`) with radius `r` and `gap`
 *  points between segments. */
export function ringArcs(marks: Mark[], c: number, r: number, gap: number, cap: number): Arc[] {
	const segs = ringSegments(marks, cap);
	if (segs.length === 1) return [{ mark: segs[0][0], d: null }];
	// Merged colours keep a visible sliver however few verses they hold.
	const floor = segs.reduce((s, [, w]) => s + w, 0) / cap;
	const weights = segs.map(([, w]) => Math.max(w, floor));
	const total = weights.reduce((s, w) => s + w, 0);
	const g = gap / r;
	const usable = 2 * Math.PI - g * segs.length;
	let a = -Math.PI / 2 + g / 2;
	const pt = (t: number) => `${(c + r * Math.cos(t)).toFixed(3)} ${(c + r * Math.sin(t)).toFixed(3)}`;
	return segs.map(([mark], i) => {
		const sweep = (usable * weights[i]) / total;
		const d = `M ${pt(a)} A ${r} ${r} 0 ${sweep > Math.PI ? 1 : 0} 1 ${pt(a + sweep)}`;
		a += sweep + g;
		return { mark, d };
	});
}
