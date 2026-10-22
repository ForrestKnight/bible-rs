/**
 * The object specs in objects/ (objects/README.md): their types, and what the page, the viewer and
 * the reader's glyph read from them. check.mjs checks the files; this trusts them.
 */
import { BOOKS, chapterPath } from '../read/books';
import { verseHref } from '../read/seo/pages';

/** The words of a verse that state a fact, in each checked translation. */
export interface Quote {
	kjv: string;
	bsb: string;
}

export interface Fact {
	text: string;
	ref: string;
	quote: Quote;
}

/** A number: stated in the text (with its verse and words) or an interpretive choice (with a note). */
export interface Dim {
	value: number;
	unit?: 'cubit' | 'count';
	interpretive: boolean;
	ref?: string;
	quote?: Quote;
	note?: string;
}

export interface Choice {
	id: string;
	label: string;
	note: string;
	values: Record<string, Dim>;
}

export interface PartOption {
	id: string;
	label: string;
	default: string;
	choices: Choice[];
}

export interface Part {
	id: string;
	label: string;
	interpretive: boolean;
	note: string;
	facts: Fact[];
	dims: Record<string, Dim>;
	options?: PartOption[];
	model: string | { file: string; author: string; license: string };
}

export interface Step {
	part: string;
	effect?: 'gild';
	ref: string;
}

export interface ObjectSpec {
	id: string;
	slug: string;
	title: string;
	summary: string;
	/** Always CC BY 4.0 (objects/LICENSE); the code that draws it is MIT. */
	license: 'CC-BY-4.0';
	sources: string[];
	unit: 'cubit';
	parts: Part[];
	assembly: Step[];
}

export interface Cubit {
	id: string;
	label: string;
	inches: number;
	cm: number;
	note: string;
	ref?: string;
	quote?: Quote;
}

export interface Cubits {
	license: 'CC-BY-4.0';
	default: string;
	cubits: Cubit[];
	person: { label: string; inches: number };
}

/** A translation's text of the verses an object cites, by OSIS reference. */
export interface VerseTexts {
	id: string;
	abbrev: string;
	name: string;
	verses: Record<string, string>;
}

/** Exod.25.10: book 0–65, chapter and verse from 1, and the chapter's index in the grid. */
export function osis(ref: string) {
	const [code, c, v] = ref.split('.');
	const book = BOOKS.find((b) => b.abbrev.replace(' ', '') === code);
	if (!book) throw new Error(`${ref}: no book called ${code}`);
	const chapter = Number(c);
	return { book: book.index, chapter, verse: Number(v.split('-')[0]), index: book.firstChapter + chapter - 1, name: book.name };
}

/** Exod.25.10 as "Exodus 25:10"; a range, Exod.25.10-22, as "Exodus 25:10–22". */
export function refLabel(ref: string): string {
	const r = osis(ref);
	const end = ref.split('-')[1];
	return `${r.name} ${r.chapter}:${r.verse}${end ? `–${end}` : ''}`;
}

/** The verse's page in a translation (or its place in the chapter, for a translation without verse pages). */
export const verseLink = (ref: string, translation: string) => {
	const r = osis(ref);
	return verseHref(BOOKS, translation, r.index, r.verse);
};

/** The chapter the verse is in, scrolled to the verse. */
export const chapterLink = (ref: string, translation: string) => {
	const r = osis(ref);
	return chapterPath(BOOKS, translation, r.index, r.verse);
};

/** Every verse the object cites, once each, in the order they're met: its sources' verses first. */
export function citedRefs(spec: ObjectSpec, cubits?: Cubits): string[] {
	const out = new Set<string>();
	for (const s of spec.sources) {
		const [head, end] = s.split('-');
		const [code, chapter, first] = head.split('.');
		for (let v = Number(first); v <= Number(end ?? first); v++) out.add(`${code}.${chapter}.${v}`);
	}
	const dims = (all: Record<string, Dim>) => Object.values(all).forEach((d) => d.ref && out.add(d.ref));
	for (const p of spec.parts) {
		p.facts.forEach((f) => out.add(f.ref));
		dims(p.dims);
		p.options?.forEach((o) => o.choices.forEach((c) => dims(c.values)));
	}
	spec.assembly.forEach((s) => out.add(s.ref));
	cubits?.cubits.forEach((c) => c.ref && out.add(c.ref));
	return [...out];
}

/** A part's verses, once each, in the order its facts give them. */
export const partRefs = (part: Part) => [...new Set(part.facts.map((f) => f.ref))];

/** A length in cubits as inches and centimetres in a cubit: "45.0 in · 114 cm". */
export function inUnits(cubitsLong: number, cubit: Cubit): { inches: string; cm: string } {
	return {
		inches: `${(cubitsLong * cubit.inches).toFixed(1)} in`,
		cm: `${Math.round(cubitsLong * cubit.cm)} cm`
	};
}

/** 2.5 as "2½", the way the text gives it. */
export const cubitsLabel = (n: number) => {
	const whole = Math.floor(n);
	const half = n - whole === 0.5;
	return `${whole || (half ? '' : '0')}${half ? '½' : ''}`;
};

/** The value of a part's option as chosen (or its default). */
export function chosen(part: Part, option: string, choice?: string): Choice | undefined {
	const o = part.options?.find((x) => x.id === option);
	return o?.choices.find((c) => c.id === (choice ?? o.default));
}
