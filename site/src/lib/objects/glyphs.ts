/**
 * The reader's glyph after a verse that describes a modelled object: the first verse of each of the
 * spec's sources (objects/ark.json), linking to the object's page. Only these three fields of the
 * spec reach the reader's scripts.
 */
import { slug, sources, title } from '../../../../objects/ark.json';
import { osis } from './spec';

export interface Glyph {
	href: string;
	label: string;
}

/** By chapter index, then verse. */
const GLYPHS = new Map<number, Map<number, Glyph>>();
for (const source of sources) {
	const r = osis(source);
	const verses = GLYPHS.get(r.index) ?? new Map<number, Glyph>();
	verses.set(r.verse, { href: `/${slug}`, label: `See ${title.replace(/^The /, 'the ')} in 3D` });
	GLYPHS.set(r.index, verses);
}

/** The glyphs in a chapter, by verse. */
export const glyphsIn = (index: number): Map<number, Glyph> | undefined => GLYPHS.get(index);
