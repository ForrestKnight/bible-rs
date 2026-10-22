import { error } from '@sveltejs/kit';
import { SITE_NAME } from '#lib/config.js';
import { bookSlug } from '#lib/read/books.js';
import { core } from '#lib/read/core.js';
import { runsText } from '#lib/read/seo/meta.js';
import { verseHref } from '#lib/read/seo/pages.js';
import { TOPIC_TRANSLATIONS, TOPICS_PATH, topicName, topicPath, topics } from '#lib/topics/index.js';
import type { EntryGenerator, PageLoad } from './$types';

/**
 * A topic from lib/topics/topics.json, each verse in the KJV and the BSB with links to its pages: a
 * plain page, no app (see the verse page's +page.ts). The verses' text comes from the core here,
 * as the page is prerendered.
 */
export const prerender = true;
export const csr = false;

export const entries: EntryGenerator = async () => (await topics()).map(({ slug }) => ({ slug }));

export const load: PageLoad = async ({ params }) => {
	const all = await topics();
	const topic = all.find((t) => t.slug === params.slug);
	if (!topic) error(404, 'No such topic');
	// Anything in the file the core disagrees with stops the build rather than printing a wrong or
	// empty verse. validate.mjs checks the same things; this is for an edit that skipped it.
	const wrong = (why: string) => new Error(`topics.json: ${topic.slug}: ${why} (run build.mjs and validate.mjs)`);
	if (!topic.title.endsWith(` | ${SITE_NAME}`)) throw wrong(`the title doesn't end in " | ${SITE_NAME}"`);
	const c = await core();
	const translations = await Promise.all(TOPIC_TRANSLATIONS.map((id) => c.load(id)));
	const verses = topic.verses.map((v) => {
		const book = c.books[v.book_index];
		if (!book || bookSlug(book) !== v.book_slug || v.chapter < 1 || v.chapter > book.chapters)
			throw wrong(`${v.ref} isn't a chapter the core has`);
		const index = book.firstChapter + v.chapter - 1;
		return {
			ref: v.ref,
			texts: translations.map((t) => {
				const runs = c.verse(t.id, { book: book.index, chapter: v.chapter, verse: v.verse })?.runs;
				if (!runs || !runsText(runs)) throw wrong(`${v.ref} isn't in the ${t.abbrev}`);
				return { id: t.id, abbrev: t.abbrev, runs, href: verseHref(c.books, t.id, index, v.verse) };
			})
		};
	});
	const path = topicPath(topic.slug);
	return {
		meta: {
			title: topic.title,
			description: topic.description,
			path,
			crumbs: [
				{ name: 'Home', path: '/' },
				{ name: 'Topics', path: TOPICS_PATH },
				{ name: topicName(topic), path }
			]
		},
		heading: `Bible Verses About ${topic.topic}`,
		intro: topic.intro,
		translations: translations.map((t) => ({ id: t.id, abbrev: t.abbrev, name: t.name, copyright: t.copyright })),
		verses,
		related: topic.related.map((slug) => {
			const r = all.find((t) => t.slug === slug);
			if (!r) throw wrong(`its related topic "${slug}" doesn't exist`);
			return { name: topicName(r), href: topicPath(slug) };
		})
	};
};
