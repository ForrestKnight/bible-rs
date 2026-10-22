import { error } from '@sveltejs/kit';
import { bookOf, bookSlug, chapterFromPath, chapterPath, versePath } from '#lib/read/books.js';
import { CHAPTERS, chapterLabel, core, type Core, type VerseContent } from '#lib/read/core.js';
import { clip, crumbs, runsText, VERSE_PAGES, verseTitle } from '#lib/read/seo/meta.js';
import { verseHref, versePages } from '#lib/read/seo/pages.js';
import { topicName, topicPath, topicsWith } from '#lib/topics/index.js';
import type { EntryGenerator, PageLoad } from './$types';

/**
 * A verse with the two before and after it, in every translation: a plain page, no app. The load
 * is universal, not a server load, only so the build writes no __data.json beside each page (with
 * no client-side rendering nothing would fetch it).
 */
export const prerender = true;
export const csr = false;

export const entries: EntryGenerator = async () =>
	(await versePages(await core())).map(({ translation, book, chapter, verse }) => ({
		translation,
		book: bookSlug(book),
		chapter: String(chapter),
		verse: String(verse)
	}));

const CONTEXT = 2;

/** The verse before or after one, across chapters, in a translation (null at either end). */
function step(c: Core, id: string, index: number, at: number, by: -1 | 1) {
	let verses = c.verses(id, index);
	let i = at + by;
	while (i < 0 || i >= verses.length) {
		index += by;
		if (index < 0 || index >= CHAPTERS) return null;
		verses = c.verses(id, index);
		i = by < 0 ? verses.length - 1 : 0;
	}
	return { index, verse: verses[i].number };
}

export const load: PageLoad = async ({ params }) => {
	const c = await core();
	const id = params.translation;
	const index = chapterFromPath(c.books, params.book, params.chapter);
	if (!VERSE_PAGES.includes(id) || !c.bundled.includes(id) || index === null) error(404, 'No such verse');
	const translations = await Promise.all(c.bundled.map((t) => c.load(t)));
	const info = translations.find((t) => t.id === id)!;
	const verses = c.verses(id, index);
	const at = verses.findIndex((v) => String(v.number) === params.verse);
	if (at < 0) error(404, 'No such verse');

	const verse = verses[at];
	const book = bookOf(c.books, index);
	const chapter = index - book.firstChapter + 1;
	const ref = { book: book.index, chapter, verse: verse.number };
	const label = `${chapterLabel(index)}:${verse.number}`;
	const text = runsText(verse.runs);
	const near = (v: VerseContent) => ({
		number: v.number,
		runs: v.runs,
		href: versePath(c.books, id, index, v.number),
		current: v === verse
	});
	const link = (to: { index: number; verse: number } | null) =>
		to && {
			label: `${chapterLabel(to.index)}:${to.verse}`,
			href: versePath(c.books, id, to.index, to.verse)
		};

	return {
		meta: {
			title: verseTitle(label, info.abbrev, text),
			description: clip(text, 300),
			path: versePath(c.books, id, index, verse.number),
			crumbs: crumbs(info, book, chapter, verse.number)
		},
		label,
		info,
		verse,
		chapter: { label: chapterLabel(index), href: chapterPath(c.books, id, index, verse.number) },
		context: verses.slice(Math.max(0, at - CONTEXT), at + CONTEXT + 1).map(near),
		prev: link(step(c, id, index, at, -1)),
		next: link(step(c, id, index, at, 1)),
		// This translation first, then the others in the reader's order. Theirs link to their own
		// verse page, or to the verse in its chapter when they have none.
		others: [info, ...translations.filter((t) => t !== info)].map((t) => {
			const v = c.verse(t.id, ref);
			return {
				id: t.id,
				abbrev: t.abbrev,
				name: t.name,
				runs: v?.runs ?? null,
				href: v ? verseHref(c.books, t.id, index, verse.number) : chapterPath(c.books, t.id, index)
			};
		}),
		topics: (await topicsWith(ref)).map((t) => ({ name: topicName(t), href: topicPath(t.slug) })),
		copy: c.copyText(id, [ref], 'reference') ?? `${text} — ${label} (${info.abbrev})`,
		// Opens the Mac app at the verse: its bible:// links take a reference as the jump field does.
		app: `bible://${encodeURI(label)}?translation=${id}`
	};
};
