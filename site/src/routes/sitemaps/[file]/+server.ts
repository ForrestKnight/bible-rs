import { error } from '@sveltejs/kit';
import { sitemaps, urlsetResponse } from '#lib/read/seo/sitemap.js';
import type { EntryGenerator, RequestHandler } from './$types';

/** The sitemap files the index at /sitemap.xml lists. */
export const prerender = true;

export const entries: EntryGenerator = async () => (await sitemaps()).map(({ file }) => ({ file }));

export const GET: RequestHandler = async ({ params }) => {
	const map = (await sitemaps()).find((s) => s.file === params.file);
	if (!map) error(404);
	return urlsetResponse(map.paths);
};
