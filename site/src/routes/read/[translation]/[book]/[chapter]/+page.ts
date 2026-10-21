import { bookSlug } from '#lib/read/books.js';
import { core } from '#lib/read/core.js';
import type { EntryGenerator } from './$types';

/** Every chapter of every bundled translation is a static page. */
export const entries: EntryGenerator = async () => {
	const c = await core();
	return c.bundled.flatMap((translation) =>
		c.books.flatMap((book) =>
			Array.from({ length: book.chapters }, (_, i) => ({
				translation,
				book: bookSlug(book),
				chapter: String(i + 1)
			}))
		)
	);
};
