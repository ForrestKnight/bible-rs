import { error } from '@sveltejs/kit';
import { BOOKS, bookFromSlug, bookPath } from '#lib/read/books.js';
import { DEFAULT_TRANSLATION } from '#lib/read/core.js';
import type { RequestHandler } from './$types';

// Served dynamically so it returns a real HTTP redirect rather than a prerendered page.
export const prerender = false;

/** Short addresses, /john/3/16 and /john/3, go to the verse's or the chapter's page in the KJV. */
export const GET: RequestHandler = ({ params }) => {
	const book = bookFromSlug(BOOKS, params.book);
	if (!book || Number(params.chapter) > book.chapters) error(404, 'No such chapter');
	const path = `${bookPath(DEFAULT_TRANSLATION, book)}/${params.chapter}${params.verse ? `/${params.verse}` : ''}`;
	return new Response(null, {
		status: 308,
		headers: { location: path, 'cache-control': 'public, max-age=3600, s-maxage=31536000' }
	});
};
