import { SITE_URL } from '#lib/config.js';
import { bookSlug } from '#lib/read/books.js';
import { core } from '#lib/read/core.js';
import type { RequestHandler } from './$types';

export const prerender = true;

/** The site's pages and every chapter of every bundled translation (8,000-odd URLs; the limit is 50,000). */
export const GET: RequestHandler = async () => {
	const c = await core();
	const paths = ['/', '/privacy', '/support', '/read'];
	for (const t of c.bundled) {
		for (const book of c.books) {
			for (let n = 1; n <= book.chapters; n++) paths.push(`/read/${t}/${bookSlug(book)}/${n}`);
		}
	}
	const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${paths.map((p) => `\t<url><loc>${SITE_URL}${p}</loc></url>`).join('\n')}
</urlset>
`;
	return new Response(body, { headers: { 'content-type': 'application/xml' } });
};
