import { error } from '@sveltejs/kit';
import { core } from '#lib/read/core.js';
import { translationBytes } from '#lib/read/node.js';
import type { EntryGenerator, RequestHandler } from './$types';

/** The bundled translations as static files (data/translations/*.bbl), fetched by the reader. */
export const prerender = true;

export const entries: EntryGenerator = async () =>
	(await core()).bundled.map((id) => ({ file: `${id}.bbl` }));

export const GET: RequestHandler = async ({ params }) => {
	const id = params.file.replace(/\.bbl$/, '');
	if (!(await core()).bundled.includes(id) || !params.file.endsWith('.bbl')) error(404);
	return new Response(translationBytes(id) as Uint8Array<ArrayBuffer>, {
		headers: { 'content-type': 'application/octet-stream' }
	});
};
