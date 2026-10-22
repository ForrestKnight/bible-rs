/**
 * The Ark's spec as the routes prerender it: checked (check.mjs) against the core's KJV and BSB
 * first, so a wrong reference or quote stops the build, then the cited verses' text from the core.
 * Server only: the check and its schema validator never reach a browser.
 */
import cubitsSchema from '../../../../objects/cubits.schema.json';
import cubitsFile from '../../../../objects/cubits.json';
import objectSchema from '../../../../objects/object.schema.json';
import arkFile from '../../../../objects/ark.json';
import { core } from '../read/core';
import { check, QUOTED } from './check.mjs';
import { citedRefs, osis, type Cubits, type ObjectSpec, type VerseTexts } from './spec';

let checked: Promise<{ spec: ObjectSpec; cubits: Cubits }> | undefined;

/** objects/ark.json and objects/cubits.json, once they've passed the check. */
export function ark(): Promise<{ spec: ObjectSpec; cubits: Cubits }> {
	checked ??= (async () => {
		const c = await core();
		await Promise.all(QUOTED.map((id) => c.load(id)));
		const { problems } = check({
			name: 'ark.json',
			spec: arkFile,
			cubits: cubitsFile,
			schemas: { object: objectSchema, cubits: cubitsSchema },
			books: c.books,
			text: (t, ref) => (c.verse(t, ref) ? c.copyText(t, [ref], 'plain') : null)
		});
		if (problems.length) throw new Error(`The object spec has problems (node site/src/lib/objects/check.mjs):\n${problems.join('\n')}`);
		return { spec: arkFile as unknown as ObjectSpec, cubits: cubitsFile as Cubits };
	})();
	return checked;
}

/** The cited verses' text in a bundled translation. */
export async function textsIn(id: string): Promise<VerseTexts> {
	const { spec, cubits } = await ark();
	const c = await core();
	const info = await c.load(id);
	const verses: Record<string, string> = {};
	for (const ref of citedRefs(spec, cubits)) {
		const r = osis(ref);
		const text = c.copyText(id, [{ book: r.book, chapter: r.chapter, verse: r.verse }], 'plain');
		if (text) verses[ref] = text;
	}
	return { id, abbrev: info.abbrev, name: info.name, verses };
}
