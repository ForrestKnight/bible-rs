import { error } from '@sveltejs/kit';
import { bookFromSlug, bookPath, bookSlug, chapterName, chapterPath } from '#lib/read/books.js';
import { core } from '#lib/read/core.js';
import { bookTitle, clip, crumbs, runsText } from '#lib/read/seo/meta.js';
import { bookPages } from '#lib/read/seo/pages.js';
import { contentOptions, defaultTypography } from '#lib/read/settings.js';
import type { EntryGenerator, PageLoad } from './$types';

/** A book's chapters, each with its opening words: a plain page, no app (see the verse page's +page.ts). */
export const prerender = true;
export const csr = false;

export const entries: EntryGenerator = async () =>
	bookPages(await core()).map(({ translation, book }) => ({ translation, book: bookSlug(book) }));

export const load: PageLoad = async ({ params }) => {
	const c = await core();
	const id = params.translation;
	const book = bookFromSlug(c.books, params.book);
	if (!c.bundled.includes(id) || !book) error(404, 'No such book');
	const translations = await Promise.all(c.bundled.map((t) => c.load(t)));
	const info = translations.find((t) => t.id === id)!;
	const first = c.chapter(id, book.firstChapter, contentOptions(defaultTypography()));
	const chapters = Array.from({ length: book.chapters }, (_, i) => {
		const index = book.firstChapter + i;
		const opening = c.verses(id, index)[0];
		return {
			n: i + 1,
			label: chapterName(c.books, index),
			href: chapterPath(c.books, id, index),
			opening: opening ? clip(runsText(opening.runs), 120) : ''
		};
	});
	const neighbour = (i: number) => {
		const b = c.books[i];
		return b && { name: b.name, href: bookPath(id, b) };
	};
	const intro = first.bookTitle && first.bookTitle !== book.name ? `${first.bookTitle}. ` : '';

	return {
		meta: {
			title: bookTitle(book, info.abbrev),
			description: clip(
				`${intro}Read the book of ${book.name} in the ${info.name}, ${book.chapters === 1 ? 'its one chapter' : `all ${book.chapters} chapters`}, free online. ${chapters[0].opening}`,
				155
			),
			path: bookPath(id, book),
			crumbs: crumbs(info, book)
		},
		info,
		book: { name: book.name, title: first.bookTitle, chapters: book.chapters },
		chapters,
		prev: neighbour(book.index - 1),
		next: neighbour(book.index + 1),
		others: translations
			.filter((t) => t !== info)
			.map((t) => ({ abbrev: t.abbrev, name: t.name, href: bookPath(t.id, book) }))
	};
};
