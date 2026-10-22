import type { Handle } from '@sveltejs/kit/hooks';

const reader = (path: string) =>
	path === '/read' || path.startsWith('/read/') || path === '/topics' || path.startsWith('/topics/');

/**
 * Reader pages (and the topic pages, set like them) carry the reader's default appearance in the
 * HTML; app.html swaps in the saved one.
 */
export const handle: Handle = ({ event, resolve }) =>
	resolve(event, {
		transformPageChunk: reader(event.url.pathname)
			? ({ html }) =>
					html
						.replace('<html lang="en">', '<html lang="en" data-read data-theme="light" data-accent="ink">')
						.replace('content="#0a0a0b"', 'content="#ffffff"')
			: undefined
	});
