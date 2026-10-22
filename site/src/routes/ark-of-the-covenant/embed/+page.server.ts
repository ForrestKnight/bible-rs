import { ark, textsIn } from '#lib/objects/server.js';
import type { PageServerLoad } from './$types';

/** The viewer alone, filling the window: what the Mac app shows in its webview. Not for search. */
export const prerender = true;

export const load: PageServerLoad = async () => {
	const { spec, cubits } = await ark();
	return { spec, cubits, kjv: await textsIn('kjv') };
};
