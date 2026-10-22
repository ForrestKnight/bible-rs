// Builds topics.json from topics.src.mjs, keeping the refs whose KJV + BSB text the repo's .bbl
// files have, read through the site's own wasm core (same code path the SvelteKit prerender uses).
// topics.json holds references only: the pages take each verse's text from the core when they're
// prerendered.
import { writeFileSync } from 'node:fs';
import { bible, books, bookSlug, siteName } from './lib.mjs';
import src from './topics.src.mjs';

const MAX = 30;
const alias = { psalm: 'Psalms', 'song of songs': 'Song of Solomon' };
const byName = new Map(books.map((b) => [b.name.toLowerCase(), b]));
const text = (t, v) => bible.copyText(t, [v], 'plain') ?? null;

function expand(s) {
  const m = s.match(/^(.+?) (\d+):([\d,]+)$/);
  if (!m) throw new Error(`bad ref ${s}`);
  const name = alias[m[1].toLowerCase()] ?? m[1];
  const book = byName.get(name.toLowerCase());
  if (!book) throw new Error(`unknown book ${m[1]}`);
  return m[3].split(',').map((v) => ({ book, chapter: +m[2], verse: +v }));
}

const dropped = [];
const out = src.map((t) => {
  const seen = new Set();
  const verses = [];
  for (const r of t.refs.flatMap(expand)) {
    const ref = `${r.book.name === 'Psalms' ? 'Psalm' : r.book.name} ${r.chapter}:${r.verse}`;
    if (seen.has(ref)) continue;
    seen.add(ref);
    const v = { book: r.book.index, chapter: r.chapter, verse: r.verse };
    const ok = r.chapter <= r.book.chapters;
    const kjv = ok ? text('kjv', v) : null, bsb = ok ? text('bsb', v) : null;
    if (!kjv || !bsb) { dropped.push(`${t.slug}: ${ref} (kjv:${!!kjv} bsb:${!!bsb})`); continue; }
    if (verses.length >= MAX) continue;
    verses.push({ ref, book: r.book.name, book_index: r.book.index, book_slug: bookSlug(r.book), chapter: r.chapter, verse: r.verse });
  }
  const N = verses.length;
  return {
    slug: t.slug, topic: t.topic,
    title: `${N} Bible Verses About ${t.topic} (KJV & BSB) | ${siteName}`,
    description: t.description.replace('{N}', N),
    intro: t.intro, related: t.related, verses,
  };
});
writeFileSync(new URL('./topics.json', import.meta.url), JSON.stringify(out, null, 2) + '\n');
console.log(`topics: ${out.length}, verses: ${out.reduce((a, t) => a + t.verses.length, 0)}`);
console.log(`dropped (unverifiable) refs: ${dropped.length}`); dropped.forEach((d) => console.log('  ' + d));
