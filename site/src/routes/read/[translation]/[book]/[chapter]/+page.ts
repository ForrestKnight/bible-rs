import { bookSlug } from '#lib/read/books.js';
import { core } from '#lib/read/core.js';
import { chapterPages } from '#lib/read/seo/pages.js';
import type { EntryGenerator } from './$types';

/** Every chapter of every bundled translation is a static page. */
export const entries: EntryGenerator = async () =>
	chapterPages(await core()).map(({ translation, book, chapter }) => ({
		translation,
		book: bookSlug(book),
		chapter: String(chapter)
	}));
