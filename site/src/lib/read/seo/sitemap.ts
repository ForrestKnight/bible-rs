/**
 * The sitemaps: /sitemap.xml is an index of the files under /sitemaps/, which list the site's own
 * pages and the translations' and books' pages, each translation's chapters, the verse pages
 * (cut into files under the protocol's 50,000 URLs) and the topics' index and pages. Every entry
 * carries the build's date.
 */
import { SITE_URL } from '#lib/config.js';
import { TOPICS_PATH, topicPath, topics } from '#lib/topics/index.js';
import { bookPath, translationPath } from '../books';
import { core } from '../core';
import { bookPages, chapterPagePath, chapterPages, versePagePath, versePages } from './pages';

/** URLs in one verse file. */
const VERSES_PER_FILE = 40_000;

export const BUILT = new Date().toISOString().slice(0, 10);

export interface Sitemap {
	/** Its name under /sitemaps/. */
	file: string;
	paths: string[];
}

let listed: Promise<Sitemap[]> | undefined;

/** Every sitemap file and the paths it lists (worked out once a build). */
export function sitemaps(): Promise<Sitemap[]> {
	listed ??= list();
	return listed;
}

async function list(): Promise<Sitemap[]> {
	const c = await core();
	const out: Sitemap[] = [
		{
			file: 'pages.xml',
			paths: [
				'/',
				'/privacy',
				'/support',
				'/ark-of-the-covenant',
				...c.bundled.map(translationPath),
				...bookPages(c).map(({ translation, book }) => bookPath(translation, book))
			]
		}
	];
	const chapters = chapterPages(c);
	for (const t of c.bundled) {
		out.push({ file: `chapters-${t}.xml`, paths: chapters.filter((p) => p.translation === t).map(chapterPagePath) });
	}
	const verses = await versePages(c);
	for (const t of new Set(verses.map((p) => p.translation))) {
		const paths = verses.filter((p) => p.translation === t).map(versePagePath);
		for (let i = 0; i * VERSES_PER_FILE < paths.length; i++) {
			out.push({ file: `verses-${t}-${i + 1}.xml`, paths: paths.slice(i * VERSES_PER_FILE, (i + 1) * VERSES_PER_FILE) });
		}
	}
	out.push({ file: 'topics.xml', paths: [TOPICS_PATH, ...(await topics()).map((t) => topicPath(t.slug))] });
	return out;
}

const xml = (body: string) =>
	new Response(`<?xml version="1.0" encoding="UTF-8"?>\n${body}`, { headers: { 'content-type': 'application/xml' } });

export const indexResponse = (files: Sitemap[]) =>
	xml(`<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${files.map((f) => `\t<sitemap><loc>${SITE_URL}/sitemaps/${f.file}</loc><lastmod>${BUILT}</lastmod></sitemap>`).join('\n')}
</sitemapindex>
`);

export const urlsetResponse = (paths: string[]) =>
	xml(`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${paths.map((p) => `\t<url><loc>${SITE_URL}${p}</loc><lastmod>${BUILT}</lastmod></url>`).join('\n')}
</urlset>
`);
