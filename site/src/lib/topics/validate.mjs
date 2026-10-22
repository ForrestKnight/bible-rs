// Validates topics.json against the repo's own data (data/translations/{kjv,bsb}.bbl via the
// site's wasm core) and against the schema/copy rules. Exit code 1 on any failure.
import { readFileSync } from 'node:fs';
import { bible, books, bookSlug, siteName } from './lib.mjs';

const topics = JSON.parse(readFileSync(new URL('./topics.json', import.meta.url)));
const errors = [], warn = [];
const err = (t, m) => errors.push(`${t}: ${m}`);
const slugs = new Set(topics.map((t) => t.slug));
let verses = 0, textChecks = 0;

if (topics.length !== 50) err('*', `expected 50 topics, got ${topics.length}`);
if (slugs.size !== topics.length) err('*', 'duplicate topic slugs');
for (const t of topics) {
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(t.slug)) err(t.slug, 'bad slug');
  const n = t.verses.length; verses += n;
  if (n < 20 || n > 30) err(t.slug, `verse count ${n} outside 20-30`);
  if (t.title !== `${n} Bible Verses About ${t.topic} (KJV & BSB) | ${siteName}`) err(t.slug, `title mismatch: ${t.title}`);
  if (t.description.length > 155) err(t.slug, `description ${t.description.length} chars > 155`);
  if (!t.description.startsWith(`${n} `)) err(t.slug, 'description count != verse count');
  const words = t.intro.trim().split(/\s+/).length;
  if (words < 60 || words > 100) err(t.slug, `intro ${words} words (want 60-100)`);
  if (t.related.length < 4 || t.related.length > 6) err(t.slug, `related count ${t.related.length}`);
  for (const r of t.related) {
    if (!slugs.has(r)) err(t.slug, `related slug "${r}" does not exist`);
    if (r === t.slug) err(t.slug, 'related includes itself');
  }
  if (new Set(t.related).size !== t.related.length) err(t.slug, 'duplicate related');
  const seen = new Set();
  for (const v of t.verses) {
    const id = `${v.book_index}:${v.chapter}:${v.verse}`;
    if (seen.has(id)) err(t.slug, `duplicate verse ${v.ref}`); seen.add(id);
    const book = books[v.book_index];
    if (!book || book.name !== v.book) { err(t.slug, `${v.ref}: bad book`); continue; }
    if (bookSlug(book) !== v.book_slug) err(t.slug, `${v.ref}: book_slug ${v.book_slug} != ${bookSlug(book)}`);
    const label = (book.name === 'Psalms' ? 'Psalm' : book.name) + ` ${v.chapter}:${v.verse}`;
    if (v.ref !== label) err(t.slug, `${v.ref}: ref label mismatch`);
    if (v.chapter < 1 || v.chapter > book.chapters) { err(t.slug, `${v.ref}: chapter out of range`); continue; }
    for (const tr of ['kjv', 'bsb']) {
      const want = bible.copyText(tr, [{ book: v.book_index, chapter: v.chapter, verse: v.verse }], 'plain');
      textChecks++;
      if (!want) { err(t.slug, `${v.ref}: not in ${tr} data`); continue; }
      // Second, independent path: the reader's own verse runs (what the topic page renders).
      const runs = bible.verse(tr, { book: v.book_index, chapter: v.chapter, verse: v.verse });
      const rendered = runs?.runs.filter((r) => !r.number).map((r) => r.text).join('').trim();
      textChecks++;
      if (rendered !== want) err(t.slug, `${v.ref}: ${tr} rendered verse runs differ from copyText`);
    }
  }
}
// Reciprocity is not required, but report one-way links for info.
const oneWay = topics.flatMap((t) => t.related.filter((r) => !topics.find((x) => x.slug === r).related.includes(t.slug)).map((r) => `${t.slug}->${r}`));
console.log(`topics: ${topics.length}`);
console.log(`verses: ${verses} (unique refs: ${new Set(topics.flatMap((t) => t.verses.map((v) => v.ref))).size})`);
console.log(`verse text checks against repo .bbl data: ${textChecks}`);
console.log(`related links: ${topics.reduce((a, t) => a + t.related.length, 0)} (one-way, informational: ${oneWay.length})`);
console.log(errors.length ? `FAIL: ${errors.length} error(s)\n` + errors.join('\n') : 'PASS: all checks passed');
process.exit(errors.length ? 1 : 0);
