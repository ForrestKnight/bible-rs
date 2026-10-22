# Bible topic pages: data for `/topics/[slug]`

50 SEO topic pages ("Bible Verses About {Topic}"), with 1,380 verse entries (975 unique references).
Every verse's KJV and BSB text comes from this repo's own data, `data/translations/{kjv,bsb}.bbl`,
read through the site's wasm core. Nothing was typed in by hand.

Written by Marketing against `56e1c91` and checked again against the tree it was added in.

## Files

| File | What it is |
|---|---|
| `topics.json` | The output: an array of 50 topic objects (schema below). |
| `topics.src.mjs` | The hand-written source: slugs, copy (description, intro, related) and reference lists. Edit this, not the JSON. |
| `build.mjs` | Expands the refs, drops any ref missing from the KJV or the BSB, caps each topic at 30, and writes `topics.json`. |
| `validate.mjs` | Checks every verse against the repo data and checks every content rule (see below). Exits 1 on failure. |
| `lib.mjs` | Loads `site/src/lib/read/wasm/bible_bg.wasm` and the two `.bbl` files in Node, from this checkout, and reads `SITE_NAME` from `site/src/lib/config.ts`. |
| `index.ts` | What the site imports: the types, `TOPICS`, the paths and the topics that list a verse. |

Rebuild and check, from this folder: `node build.mjs && node validate.mjs`. The site's type check
skips the `.mjs` files (`site/tsconfig.json`).

## Schema

```ts
type Topic = {
  slug: string;          // URL segment: /topics/{slug}, e.g. "gods-love", "new-beginnings"
  topic: string;         // display name, e.g. "God's Love", "the Holy Spirit", "Grief and Loss"
  title: string;         // <title>: "{N} Bible Verses About {topic} (KJV & BSB) | Simply My Bible", N = verses.length
  description: string;   // meta description, at most 155 chars, starts with N
  intro: string;         // 60-100 words of original copy (plain text; may contain straight quotes)
  related: string[];     // 4-6 slugs, all guaranteed to exist in this file, never the page's own slug
  verses: Verse[];       // 20-30 verses, in the curated display order
};

type Verse = {
  ref: string;           // display label, e.g. "John 3:16", "Psalm 23:4" (singular "Psalm" for display)
  book: string;          // core book name, e.g. "Psalms", "Song of Solomon", "1 Corinthians"
  book_index: number;    // 0-65, same as the core's Book.index / VerseRef.book
  book_slug: string;     // bookSlug(book) from site/src/lib/read/books.ts, e.g. "song-of-solomon"
  chapter: number;       // 1-based
  verse: number;         // 1-based
};
```

The title's last part is the site's name, `SITE_NAME` in `site/src/lib/config.ts`. If that changes,
run `build.mjs` again: the route stops the build on a title that doesn't end in it.

## Verse text

The file holds references only. The route (`site/src/routes/topics/[slug]/+page.ts`) takes each
verse's text from the core when the page is prerendered, as the verse pages do:

```ts
import { core } from '#lib/read/core.js';
const c = await core();                 // during SSR, this reads ../data/translations/*.bbl from disk (node.ts)
await c.load('kjv'); await c.load('bsb');
const v = { book: verse.book_index, chapter: verse.chapter, verse: verse.verse };
const runs = c.verse('kjv', v)?.runs;   // styled runs: red letters, small caps, italics
```

Joining the runs that aren't verse numbers gives the same string as `c.copyText('kjv', [v], 'plain')`.
The build stops on a reference the core doesn't have in either translation.

## Links

Each verse links to its verse page in both translations, `/read/{translation}/{book_slug}/{chapter}/{verse}`,
for example `/read/kjv/john/3/16` and `/read/bsb/psalms/23/4`. A translation without verse pages
(`VERSE_PAGES` in `site/src/lib/read/seo/meta.ts`) gets the verse's place in its chapter instead,
`/read/{translation}/{book_slug}/{chapter}#v{verse}`. Every verse page links back to the topics
that list its verse, `/topics` lists every topic, and the topics sitemap (`/sitemaps/topics.xml`)
names the index and the 50 pages.

## What validate.mjs checks

- There are exactly 50 topics, and the slugs are unique and kebab-case.
- Each topic has 20-30 verses, with no duplicate verses inside a topic.
- `title` matches the exact template with N equal to `verses.length`. `description` is at most 155 chars and starts with N.
- `intro` is 60-100 words.
- `related` has 4-6 entries. Every entry exists, none is the page's own slug, and none repeats.
- `book_index`, `book`, `book_slug` and `ref` all agree with the core's book table and `bookSlug()`.
- Each chapter is within range.
- Each verse is in both the KJV and the BSB, checked two independent ways that must agree:
  `copyText(..., 'plain')` and the reader's rendered verse runs.

Related links don't have to be reciprocal. 52 of the 266 links are one-way, which is fine.
