/**
 * The cited verses from a translation imported into this browser's reader: through the core, which
 * the Ark page loads only for this (texts.ts).
 */
import { importedInfos } from '../read/imports';
import { core } from '../read/core';
import { osis, type VerseTexts } from './spec';

export async function importedTexts(translation: string, refs: string[]): Promise<VerseTexts | null> {
	const info = importedInfos().find((t) => t.id === translation);
	if (!info) return null;
	try {
		const c = await core();
		await c.load(translation);
		const verses: Record<string, string> = {};
		for (const ref of refs) {
			const r = osis(ref);
			const text = c.copyText(translation, [{ book: r.book, chapter: r.chapter, verse: r.verse }], 'plain');
			if (text) verses[ref] = text;
		}
		return { id: translation, abbrev: info.abbrev, name: info.name, verses };
	} catch {
		return null;
	}
}
