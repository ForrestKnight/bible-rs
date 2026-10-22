import { version } from '$app/env';
import { assets, immutable } from '$app/manifest';
import { self } from '$app/service-worker';

/**
 * The reader works offline. Its scripts, wasm and fonts are cached when the worker installs (from
 * the reader only, see Shell.svelte); translations and reader pages the first time they load, and
 * the importers' module the first time a translation is imported. Each deploy starts a fresh
 * cache, since a page refers to its own build's scripts. Nothing from /api (the NLT, read live) is
 * kept: Tyndale's terms allow no more than a chapter at a time (src/lib/read/nlt.ts). The object
 * pages' 3D viewer isn't cached either: the reader never uses it.
 */
const CACHE = `read-${version}`;
/** The viewer's files, which vite.config.ts names as it builds the worker (none in development). */
declare const __VIEWER_FILES__: string[];
const VIEWER = new Set<string>(typeof __VIEWER_FILES__ === 'undefined' ? [] : __VIEWER_FILES__);
const PRECACHE = [
	...immutable.filter((f) => !f.path.includes('bible_import') && !VIEWER.has(f.path)).map((f) => `/${f.path}`),
	...assets.filter((f) => f.path.startsWith('fonts/read/') && f.path.endsWith('.woff2')).map((f) => `/${f.path}`),
	'/favicon.png',
	'/read'
];
const precached = new Set(PRECACHE);

self.addEventListener('install', (event) => {
	event.waitUntil(
		caches
			.open(CACHE)
			.then((cache) => cache.addAll(PRECACHE))
			.then(() => self.skipWaiting())
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches
			.keys()
			.then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
			.then(() => self.clients.claim())
	);
});

// The translations a page loaded before this worker controlled it.
self.addEventListener('message', (event) => {
	const urls: unknown = event.data?.cache;
	if (!Array.isArray(urls)) return;
	const wanted = urls.filter((u): u is string => typeof u === 'string' && u.startsWith('/translations/'));
	event.waitUntil(
		caches.open(CACHE).then(async (cache) => {
			for (const url of wanted) if (!(await cache.match(url))) await cache.add(url).catch(() => {});
		})
	);
});

self.addEventListener('fetch', (event) => {
	const request = event.request;
	const url = new URL(request.url);
	if (request.method !== 'GET' || url.origin !== self.location.origin) return;
	const path = url.pathname;
	if (
		path.startsWith('/translations/') ||
		path.includes('bible_import') ||
		(precached.has(path) && path !== '/read')
	) {
		event.respondWith(cacheFirst(request));
	} else if (path === '/read' || path.startsWith('/read/')) {
		event.respondWith(networkFirst(request));
	}
});

async function cacheFirst(request: Request): Promise<Response> {
	const cache = await caches.open(CACHE);
	const hit = await cache.match(request);
	if (hit) return hit;
	const response = await fetch(request);
	if (response.ok) void cache.put(request, response.clone());
	return response;
}

/** Pages come from the network when it answers; offline, a page read before, or the reader. */
async function networkFirst(request: Request): Promise<Response> {
	const cache = await caches.open(CACHE);
	try {
		const response = await fetch(request);
		if (response.ok) void cache.put(request, response.clone());
		return response;
	} catch (e) {
		const hit = (await cache.match(request)) ?? (request.mode === 'navigate' ? await cache.match('/read') : undefined);
		if (hit) return hit;
		throw e;
	}
}
