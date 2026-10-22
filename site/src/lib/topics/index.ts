/**
 * Topic pages (/topics/<slug>, "27 Bible Verses About Anxiety"): Marketing's copy and verse lists
 * in topics.json, which build.mjs makes from topics.src.mjs and validate.mjs checks (README.md).
 * The file holds references only; the route takes each verse's KJV and BSB text from the core when
 * it prerenders the page. /topics lists them, the topics sitemap names them, and a verse's own page
 * links to the topics that list it.
 */
import type { VerseRef } from '#lib/read/core.js';

export interface TopicVerse {
	/** "John 3:16", "Psalm 23:4". */
	ref: string;
	/** The core's book name, "Psalms", "Song of Solomon". */
	book: string;
	/** The core's Book.index, 0 to 65. */
	book_index: number;
	/** bookSlug(book), "song-of-solomon". */
	book_slug: string;
	/** From 1. */
	chapter: number;
	/** From 1. */
	verse: number;
}

export interface Topic {
	/** The address, /topics/<slug>: lower case, digits and hyphens. */
	slug: string;
	/** "Anxiety", "God's Love", "the Holy Spirit": the words after "Bible Verses About". */
	topic: string;
	/** The page's whole title, ending in " | " and SITE_NAME. */
	title: string;
	/** The search result's description, at most 155 characters. */
	description: string;
	/** A paragraph of plain text under the heading. */
	intro: string;
	/** Other topics' slugs. */
	related: string[];
	/** In the order the page lists them. */
	verses: TopicVerse[];
}

/** The translations a topic page prints each verse in (its title says "KJV & BSB"). */
export const TOPIC_TRANSLATIONS = ['kjv', 'bsb'];

export const TOPICS_PATH = '/topics';
export const topicPath = (slug: string) => `${TOPICS_PATH}/${slug}`;

/** The topic as a link's or a list's words: "The Holy Spirit". */
export const topicName = (t: Topic) => t.topic[0].toUpperCase() + t.topic.slice(1);

const key = (v: VerseRef) => `${v.book}:${v.chapter}:${v.verse}`;

let read: Promise<{ list: Topic[]; byVerse: Map<string, Topic[]> }> | undefined;

/**
 * topics.json, on the server only. The pages that use it have no client-side code, and keeping the
 * file out of the client's scripts keeps it out of what the reader's service worker caches.
 */
function data() {
	read ??= (async () => {
		const byVerse = new Map<string, Topic[]>();
		if (!import.meta.env.SSR) return { list: [], byVerse };
		const list: Topic[] = (await import('./topics.json')).default;
		for (const t of list) {
			for (const v of t.verses) {
				const k = key({ book: v.book_index, chapter: v.chapter, verse: v.verse });
				byVerse.set(k, [...(byVerse.get(k) ?? []), t]);
			}
		}
		return { list, byVerse };
	})();
	return read;
}

/** Every topic, in topics.json's order. */
export const topics = async () => (await data()).list;

/** The topics that list a verse, in topics.json's order. */
export const topicsWith = async (v: VerseRef) => (await data()).byVerse.get(key(v)) ?? [];
