import { DOWNLOAD_URL } from '#lib/config.js';
import type { RequestHandler } from './$types';

// Served dynamically so it returns a real HTTP 302 rather than a prerendered page.
export const prerender = false;

// A plain Response is used because SvelteKit's redirect() rejects external URLs by default.
export const GET: RequestHandler = () =>
	new Response(null, {
		status: 302,
		headers: { location: DOWNLOAD_URL, 'cache-control': 'no-store' }
	});
