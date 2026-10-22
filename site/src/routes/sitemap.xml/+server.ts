import { indexResponse, sitemaps } from '#lib/read/seo/sitemap.js';
import type { RequestHandler } from './$types';

export const prerender = true;

/** The sitemap index: one file per kind of page (lib/read/seo/sitemap.ts). */
export const GET: RequestHandler = async () => indexResponse(await sitemaps());
