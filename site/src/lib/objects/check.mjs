/**
 * Checks the object specs in objects/ (objects/README.md): each file against its JSON Schema, then
 * what a schema can't see. Every reference has to name a verse the repository's KJV and BSB both
 * have, every quote has to be in its verse in that translation, and the ids, the assembly and the
 * options have to agree.
 *
 * The Ark page runs it as it's prerendered, so a bad spec stops the build. CI runs it on its own:
 * `node site/src/lib/objects/check.mjs [folder]` prints what it checked, or every problem and exits 1.
 */
import Ajv2020 from 'ajv/dist/2020.js';

/** The translations every quote is checked against. */
export const QUOTED = ['kjv', 'bsb'];

/** The generators a part's `procedural:<name>` model can name (lib/objects/scene.ts draws them). */
export const GENERATORS = ['chest', 'crown', 'rings', 'poles', 'tablets', 'slab', 'cherub', 'glow'];

/**
 * What lib/objects/scene.ts reads from each object it draws: every part by id, with these dims,
 * and these options with a number each choice sets. A spec without them would pass its schema
 * and then fail to draw.
 */
export const SCENE = {
	ark: {
		parts: {
			chest: ['length', 'breadth', 'height', 'gold'],
			crown: ['height', 'thickness'],
			rings: ['radius', 'thickness', 'inset'],
			poles: ['length', 'radius'],
			testimony: ['width', 'height', 'thickness'],
			'mercy-seat': ['length', 'breadth', 'thickness'],
			cherubim: ['height', 'inset'],
			presence: []
		},
		options: { 'rings.position': 'height' }
	}
};

/**
 * @typedef {{ index: number; abbrev: string; chapters: number }} Book
 * @typedef {{ book: number; chapter: number; verse: number }} VerseRef
 * @typedef {(translation: string, ref: VerseRef) => string | null} VerseText
 */

/** Exod.25.10 (or the range Exod.25.10-22) as verses in the core's grid; null if no book has that OSIS name. */
export function parseOsis(/** @type {string} */ osis, /** @type {Book[]} */ books) {
	const m = /^([1-3]?[A-Z][a-z]+)\.(\d+)\.(\d+)(?:-(\d+))?$/.exec(osis);
	if (!m) return null;
	// The core's abbreviations are OSIS names with a space after a book's number ("1 Kgs").
	const book = books.find((b) => b.abbrev.replace(' ', '') === m[1]);
	if (!book) return null;
	const chapter = Number(m[2]);
	const first = Number(m[3]);
	const last = m[4] ? Number(m[4]) : first;
	/** @type {VerseRef[]} */
	const verses = [];
	for (let verse = first; verse <= last; verse++) verses.push({ book: book.index, chapter, verse });
	return verses;
}

const NUMBERS = { one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10, twelve: 12 };

/**
 * The number a quote states: "two cubits and a half" and "two and a half cubits" are 2.5, "a cubit
 * and a half" is 1.5, "four rings of gold" is 4. Null when it states none.
 */
export function statedNumber(/** @type {string} */ quote) {
	const words = quote.toLowerCase().match(/[a-z]+|\d+(?:\.\d+)?/g) ?? [];
	const at = words.findIndex((w, i) => w in NUMBERS || /^\d/.test(w) || ((w === 'a' || w === 'an') && /^cubits?$/.test(words[i + 1] ?? '')));
	if (at < 0) return null;
	const w = words[at];
	const n = /** @type {Record<string, number>} */ (NUMBERS)[w] ?? (/^\d/.test(w) ? Number(w) : 1);
	return words.slice(at).join(' ').includes('and a half') ? n + 0.5 : n;
}

/** Curly and straight quotes, and runs of spaces, count as the same. */
const plain = (/** @type {string} */ s) => s.replace(/[‘’]/g, "'").replace(/[“”]/g, '"').replace(/\s+/g, ' ').trim();

/**
 * Every problem with one object spec and the shared cubits file, as messages; none means it passed.
 * `schemas` are object.schema.json and cubits.schema.json; `text` gives a verse's text in a
 * translation, or null when it has no such verse.
 *
 * @param {{ name: string; spec: any; cubits: any; schemas: { object: any; cubits: any }; books: Book[]; text: VerseText }} input
 */
export function check({ name, spec, cubits, schemas, books, text }) {
	/** @type {string[]} */
	const problems = [];
	const ajv = new Ajv2020({ allErrors: true, strict: true, strictRequired: false });
	ajv.addSchema(schemas.object, 'object.schema.json');
	ajv.addSchema(schemas.cubits, 'cubits.schema.json');
	for (const [file, data, schema] of [
		[name, spec, 'object.schema.json'],
		['cubits.json', cubits, 'cubits.schema.json']
	]) {
		const validate = ajv.getSchema(schema);
		if (validate && !validate(data)) {
			for (const e of validate.errors ?? []) problems.push(`${file}: ${e.instancePath || '/'} ${e.message}`);
		}
	}
	// The rest walks the file, which the schema has to have let through.
	if (problems.length) return { problems, refs: 0, quotes: 0 };

	let refs = 0;
	let quotes = 0;
	/** The verse, which every quoted translation must have. */
	const verse = (/** @type {string} */ where, /** @type {string} */ osis) => {
		const verses = parseOsis(osis, books);
		if (!verses) {
			problems.push(`${where}: ${osis} names no book (use the OSIS name, like Exod or 1Kgs)`);
			return null;
		}
		const book = books[verses[0].book];
		if (verses[0].chapter > book.chapters) {
			problems.push(`${where}: ${osis}: ${book.abbrev} has ${book.chapters} chapters`);
			return null;
		}
		for (const v of verses) {
			refs++;
			for (const t of QUOTED) {
				if (!text(t, v)) problems.push(`${where}: ${osis}: the ${t.toUpperCase()} has no verse ${v.chapter}:${v.verse}`);
			}
		}
		return verses;
	};
	const quoted = (/** @type {string} */ where, /** @type {string} */ osis, /** @type {Record<string, string>} */ quote) => {
		const verses = verse(where, osis);
		if (!verses) return;
		for (const t of QUOTED) {
			const words = text(t, verses[0]);
			if (words === null) continue;
			quotes++;
			if (!plain(words).includes(plain(quote[t])))
				problems.push(`${where}: "${quote[t]}" isn't in ${osis} in the ${t.toUpperCase()}, which reads: "${plain(words)}"`);
		}
	};
	const dims = (/** @type {string} */ where, /** @type {Record<string, any>} */ all) => {
		for (const [key, d] of Object.entries(all)) {
			if (d.ref && d.quote) quoted(`${where}.${key}`, d.ref, d.quote);
			else if (d.ref) verse(`${where}.${key}`, d.ref);
			// A stated number has to be the one its words give, in each translation.
			if (!d.interpretive && d.quote)
				for (const t of QUOTED) {
					const n = statedNumber(d.quote[t]);
					if (n !== d.value)
						problems.push(`${where}.${key}: its value is ${d.value}, but the ${t.toUpperCase()}'s words "${d.quote[t]}" give ${n ?? 'no number'}`);
				}
		}
	};

	spec.sources.forEach((/** @type {string} */ s, /** @type {number} */ i) => verse(`${name}: sources[${i}]`, s));
	const ids = new Set();
	for (const part of spec.parts) {
		const at = `${name}: ${part.id}`;
		if (ids.has(part.id)) problems.push(`${at}: another part has this id`);
		ids.add(part.id);
		const model = typeof part.model === 'string' ? part.model.replace(/^procedural:/, '') : null;
		if (model !== null && !GENERATORS.includes(model))
			problems.push(`${at}: no generator called "${model}" (there are ${GENERATORS.join(', ')})`);
		part.facts.forEach((/** @type {any} */ f, /** @type {number} */ i) => quoted(`${at}.facts[${i}]`, f.ref, f.quote));
		dims(`${at}.dims`, part.dims);
		for (const o of part.options ?? []) {
			const choices = o.choices.map((/** @type {any} */ c) => c.id);
			if (!choices.includes(o.default)) problems.push(`${at}.options.${o.id}: its default "${o.default}" isn't one of its choices`);
			if (new Set(choices).size !== choices.length) problems.push(`${at}.options.${o.id}: two choices share an id`);
			for (const c of o.choices) dims(`${at}.options.${o.id}.${c.id}`, c.values);
		}
	}
	const scene = /** @type {Record<string, { parts: Record<string, string[]>; options: Record<string, string> }>} */ (SCENE)[spec.id];
	if (!scene) problems.push(`${name}: scene.ts has no model for "${spec.id}" (add it to SCENE in check.mjs once it does)`);
	else {
		for (const [id, keys] of Object.entries(scene.parts)) {
			const part = spec.parts.find((/** @type {any} */ p) => p.id === id);
			if (!part) problems.push(`${name}: scene.ts draws a part "${id}", which the spec doesn't have`);
			else for (const k of keys) if (!part.dims[k]) problems.push(`${name}: ${id}: scene.ts needs its "${k}"`);
		}
		for (const p of spec.parts) if (!scene.parts[p.id]) problems.push(`${name}: ${p.id}: scene.ts doesn't draw a part with this id`);
		for (const [key, value] of Object.entries(scene.options)) {
			const [id, option] = key.split('.');
			const o = spec.parts.find((/** @type {any} */ p) => p.id === id)?.options?.find((/** @type {any} */ x) => x.id === option);
			if (!o) problems.push(`${name}: ${id}: scene.ts needs the option "${option}"`);
			else for (const c of o.choices) if (!c.values[value]) problems.push(`${name}: ${id}.options.${option}.${c.id}: scene.ts needs its "${value}"`);
		}
	}
	spec.assembly.forEach((/** @type {any} */ step, /** @type {number} */ i) => {
		const at = `${name}: assembly[${i}]`;
		if (!ids.has(step.part)) problems.push(`${at}: no part called "${step.part}"`);
		verse(at, step.ref);
	});
	if (!spec.assembly.some((/** @type {any} */ s) => s.part === spec.parts[0].id))
		problems.push(`${name}: the assembly never builds the first part, ${spec.parts[0].id}`);

	const units = cubits.cubits.map((/** @type {any} */ c) => c.id);
	if (!units.includes(cubits.default)) problems.push(`cubits.json: the default "${cubits.default}" isn't one of its cubits`);
	for (const c of cubits.cubits) {
		if (Math.abs(c.inches * 2.54 - c.cm) > 0.01) problems.push(`cubits.json: ${c.id}: ${c.inches} in isn't ${c.cm} cm`);
		if (c.ref) quoted(`cubits.json: ${c.id}`, c.ref, c.quote);
	}
	return { problems, refs, quotes };
}

/** objects/*.json (or another folder's, named as the argument), checked against the KJV and BSB in data/translations. */
async function main() {
	const { readFileSync, readdirSync } = await import('node:fs');
	const { fileURLToPath } = await import('node:url');
	const here = (/** @type {string} */ p) => fileURLToPath(new URL(p, import.meta.url));
	const json = (/** @type {string} */ p) => JSON.parse(readFileSync(p, 'utf8'));
	const dir = process.argv[2] ? `${process.argv[2].replace(/\/$/, '')}/` : here('../../../../objects/');
	const wasm = await import('../read/wasm/bible.js');
	wasm.initSync({ module: readFileSync(here('../read/wasm/bible_bg.wasm')) });
	const bible = new wasm.Bible();
	for (const t of QUOTED) bible.load(readFileSync(here(`../../../../data/translations/${t}.bbl`)));
	/** @type {VerseText} */
	const text = (t, ref) => (bible.has(t) && bible.verse(t, ref) ? (bible.copyText(t, [ref], 'plain') ?? null) : null);
	const schemas = { object: json(`${dir}object.schema.json`), cubits: json(`${dir}cubits.schema.json`) };
	const cubits = json(`${dir}cubits.json`);
	const files = readdirSync(dir).filter((f) => f.endsWith('.json') && !f.endsWith('.schema.json') && f !== 'cubits.json');
	let failed = false;
	for (const name of files) {
		const { problems, refs, quotes } = check({ name, spec: json(dir + name), cubits, schemas, books: wasm.books(), text });
		for (const p of problems) console.error(`objects/${p}`);
		if (problems.length) failed = true;
		else console.log(`objects/${name}: ok, ${refs} verse references and ${quotes} quotes checked in the ${QUOTED.map((t) => t.toUpperCase()).join(' and ')}`);
	}
	if (!files.length) console.log('objects/: no object specs');
	if (failed) process.exit(1);
}

if (typeof process !== 'undefined' && process.argv[1]) {
	const { realpathSync } = await import('node:fs');
	const { pathToFileURL } = await import('node:url');
	if (pathToFileURL(realpathSync(process.argv[1])).href === import.meta.url) await main();
}
