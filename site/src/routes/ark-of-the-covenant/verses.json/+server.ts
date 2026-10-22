import { json } from '@sveltejs/kit';
import { core } from '#lib/read/core.js';
import { textsIn } from '#lib/objects/server.js';
import type { RequestHandler } from './$types';

/**
 * The Ark's cited verses in every bundled translation, for the viewer's hotspots in the reader's
 * current translation (lib/objects/texts.ts). The Mac app answers the same address itself.
 */
export const prerender = true;

export const GET: RequestHandler = async () => {
	const c = await core();
	return json({ translations: await Promise.all(c.bundled.map(textsIn)) });
};
