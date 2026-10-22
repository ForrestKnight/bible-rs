import { error } from '@sveltejs/kit';
import { bookPath, chapterPath, translationPath } from '#lib/read/books.js';
import { chapterLabel, core } from '#lib/read/core.js';
import { clip, crumbs, translationTitle } from '#lib/read/seo/meta.js';
import type { EntryGenerator, PageLoad } from './$types';

/** A translation's books, by testament: a plain page, no app (see the verse page's +page.ts). */
export const prerender = true;
export const csr = false;

export const entries: EntryGenerator = async () => (await core()).bundled.map((translation) => ({ translation }));

/** Where "Start reading" offers to begin: Genesis 1, Psalm 23 and John 1 (book index, chapter). */
const STARTS = [
	[0, 1],
	[18, 23],
	[42, 1]
];

export const load: PageLoad = async ({ params }) => {
	const c = await core();
	const id = params.translation;
	if (!c.bundled.includes(id)) error(404, 'No such translation');
	const translations = await Promise.all(c.bundled.map((t) => c.load(t)));
	const info = translations.find((t) => t.id === id)!;
	const testament = (nt: boolean) =>
		c.books
			.filter((b) => b.newTestament === nt)
			.map((b) => ({ name: b.name, chapters: b.chapters, href: bookPath(id, b) }));
	const year = info.year ? `, ${info.year}` : '';

	return {
		meta: {
			title: translationTitle(info.name, info.abbrev),
			description: clip(
				`Read the ${info.name} (${info.abbrev}${year}) free online: all 66 books, chapter by chapter. ${info.license}.`,
				155
			),
			path: translationPath(id),
			crumbs: crumbs(info)
		},
		info,
		starts: STARTS.map(([book, chapter]) => {
			const index = c.books[book].firstChapter + chapter - 1;
			return { label: chapterLabel(index), href: chapterPath(c.books, id, index) };
		}),
		testaments: [
			{ name: 'Old Testament', books: testament(false) },
			{ name: 'New Testament', books: testament(true) }
		],
		others: translations
			.filter((t) => t !== info)
			.map((t) => ({ abbrev: t.abbrev, name: t.name, href: translationPath(t.id) }))
	};
};
