import { readFileSync, writeFileSync } from 'node:fs';

/**
 * adapter-vercel gives every prerendered page two routes (`/a` rewritten to `/a/`, and `/a/`
 * redirected to `/a`), and Vercel refuses more than 2,048. The site prerenders about 100,000
 * pages (chapters and verses), so this folds those pairs into one pair per pattern, matching
 * exactly the same paths: pages whose paths differ in one segment only share a pattern,
 * `/read/(kjv|web)/john/(1|…|21)`.
 *
 * @param {string} file the adapter's `.vercel/output/config.json`
 */
export function compactRoutes(file) {
	const config = JSON.parse(readFileSync(file, 'utf8'));
	/** @type {any[]} */
	const routes = config.routes;
	/** @type {any[]} the routes that aren't a page's pair */
	const others = [];
	/** @type {string[]} */
	const paths = [];
	let first = -1;
	for (let i = 0; i < routes.length; i++) {
		const [a, b] = [routes[i], routes[i + 1]];
		const page =
			b !== undefined &&
			Object.keys(a).length === 2 &&
			typeof a.src === 'string' &&
			!a.src.endsWith('/') &&
			a.dest === `${a.src}/` &&
			b.src === a.dest &&
			b.status === 308 &&
			b.headers?.Location === a.src &&
			Object.keys(b).length === 3;
		if (!page) {
			others.push(a);
			continue;
		}
		if (first < 0) first = others.length;
		paths.push(a.src);
		i++;
	}
	if (!paths.length) return;

	/** @type {string[][][]} each pattern: the choices for each segment */
	let patterns = paths.map((p) => p.slice(1).split('/').map((s) => [s]));
	const depths = patterns.reduce((n, p) => Math.max(n, p.length), 0);
	for (let pass = 0; pass < 2; pass++) {
		for (let at = depths - 1; at >= 0; at--) patterns = merge(patterns, at);
	}
	const count = patterns.reduce((n, p) => n + p.reduce((m, s) => m * s.length, 1), 0);
	if (count !== new Set(paths).size) throw new Error(`compactRoutes: ${count} paths for ${paths.length} pages`);

	const pairs = patterns.flatMap((p) => {
		const path = p.map((s) => (s.length === 1 ? escape(s[0]) : `(?:${s.map(escape).join('|')})`)).join('/');
		return [
			{ src: `^(/${path})$`, dest: '$1/' },
			{ src: `^(/${path})/$`, status: 308, headers: { Location: '$1' } }
		];
	});
	others.splice(first, 0, ...pairs);
	config.routes = others;
	writeFileSync(file, JSON.stringify(config, null, '\t'));
}

/**
 * Adds headers to every response whose path matches `src`, ahead of the adapter's routes.
 *
 * @param {string} file the adapter's `.vercel/output/config.json`
 * @param {string} src a route pattern
 * @param {Record<string, string>} headers
 */
export function addHeaders(file, src, headers) {
	const config = JSON.parse(readFileSync(file, 'utf8'));
	config.routes.unshift({ src, headers, continue: true });
	writeFileSync(file, JSON.stringify(config, null, '\t'));
}

/**
 * Joins patterns that are the same everywhere but segment `at`.
 *
 * @param {string[][][]} patterns
 * @param {number} at
 */
function merge(patterns, at) {
	/** @type {Map<string, string[][]>} */
	const joined = new Map();
	for (const p of patterns) {
		if (p.length <= at) {
			joined.set(JSON.stringify(p), p);
			continue;
		}
		const key = JSON.stringify(p.map((s, i) => (i === at ? null : s)));
		const into = joined.get(key);
		if (into) into[at].push(...p[at]);
		else joined.set(key, p.map((s) => [...s]));
	}
	return [...joined.values()];
}

/** @param {string} s */
function escape(s) {
	return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
