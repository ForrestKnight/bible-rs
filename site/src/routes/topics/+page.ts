import { SITE_NAME } from '#lib/config.js';
import { core } from '#lib/read/core.js';
import { TOPIC_TRANSLATIONS, TOPICS_PATH, topicName, topicPath, topics } from '#lib/topics/index.js';
import type { PageLoad } from './$types';

/** Every topic, linked: a plain page, no app (see the verse page's +page.ts). */
export const prerender = true;
export const csr = false;

export const load: PageLoad = async () => {
	const all = await topics();
	const c = await core();
	const translations = await Promise.all(TOPIC_TRANSLATIONS.map((id) => c.load(id)));
	const abbrevs = translations.map((t) => t.abbrev);
	const verses = all.reduce((n, t) => n + t.verses.length, 0).toLocaleString('en-US');
	return {
		meta: {
			title: `Bible Verses by Topic (${abbrevs.join(' & ')}) | ${SITE_NAME}`,
			description: `Bible verses by topic in the ${abbrevs.join(' and ')}: ${verses} verses on ${all.length} topics, including anxiety, grief, hope, love and forgiveness. Free to read.`,
			path: TOPICS_PATH,
			crumbs: [
				{ name: 'Home', path: '/' },
				{ name: 'Topics', path: TOPICS_PATH }
			]
		},
		summary: `${all.length} topics and ${verses} verses, each in the ${translations.map((t) => `${t.name} (${t.abbrev})`).join(' and the ')}`,
		topics: all.map((t) => ({ name: topicName(t), href: topicPath(t.slug), count: t.verses.length }))
	};
};
