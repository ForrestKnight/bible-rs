import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
// The repository's root, four levels up from site/src/lib/topics.
const R = fileURLToPath(new URL('../../../..', import.meta.url)).replace(/\/$/, '');
const wasm = await import(R + '/site/src/lib/read/wasm/bible.js');
wasm.initSync({ module: readFileSync(R + '/site/src/lib/read/wasm/bible_bg.wasm') });
export const bible = new wasm.Bible();
export const books = wasm.books();
export const bundled = wasm.bundledTranslations();
for (const t of ['kjv', 'bsb']) bible.load(readFileSync(`${R}/data/translations/${t}.bbl`));
// The site's name, which ends every title: SITE_NAME in site/src/lib/config.ts.
export const siteName = readFileSync(`${R}/site/src/lib/config.ts`, 'utf8').match(/SITE_NAME = '([^']+)'/)?.[1];
if (!siteName) throw new Error('SITE_NAME not found in site/src/lib/config.ts');
export const bookSlug = (b) => b.name.toLowerCase().replace(/\s+/g, '-');
export const wasmMod = wasm;
export function verseText(t, ref) { // ref {book, chapter, verse}
  const v = bible.verse(t, ref);
  if (!v) return null;
  return v.runs.filter(r => !r.number && !r.added).map(r => r.text).join('');
}
