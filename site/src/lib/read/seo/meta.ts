/**
 * What search engines and link previews read on the reader's pages: titles, descriptions,
 * breadcrumbs and their JSON-LD, made the same way for a chapter (the reader itself), a verse, a
 * book and a translation. Absolute URLs come from SITE_URL.
 */
import { SITE_NAME, SITE_URL } from '#lib/config.js';
import { bookPath, translationPath } from '../books';
import type { Book, Run } from '../core';

/**
 * Translations whose verses have pages of their own (/read/kjv/john/3/16). The others' verses are
 * linked in their chapter (#v16). DECISIONS.md (Web) has the numbers: all seven would make the
 * build about twice the size and three times as slow.
 */
export const VERSE_PAGES = ['kjv', 'bsb', 'web'];

/** A verse's words without its number or the layout's added spaces. */
export const runsText = (runs: Run[]) =>
	runs
		.filter((r) => !r.number)
		.map((r) => r.text)
		.join('')
		.replace(/\s+/g, ' ')
		.trim();

/** Text cut to at most `max` characters at a word break, ending in an ellipsis when cut. */
export function clip(text: string, max: number): string {
	const t = text.replace(/\s+/g, ' ').trim();
	if (t.length <= max) return t;
	let cut = t.slice(0, max - 1);
	if (!/\s/.test(t[max - 1])) cut = cut.replace(/\s+\S*$/, '');
	return `${cut.replace(/[\s,;:.—–-]+$/, '')}…`;
}

const TITLE = 60;

/** What ends every title. */
const BRAND = ` | ${SITE_NAME}`;

export const chapterTitle = (label: string, abbrev: string) => `${label} ${abbrev} – Read the Full Chapter${BRAND}`;

export const bookTitle = (book: Book, abbrev: string) =>
	`The Book of ${book.name} (${abbrev}) – ${book.chapters === 1 ? '1 Chapter' : `All ${book.chapters} Chapters`}${BRAND}`;

export const translationTitle = (name: string, abbrev: string) => `${name} (${abbrev}) – All 66 Books${BRAND}`;

/**
 * `John 3:16 KJV – "For God so loved the…" | Simply My Bible`: as many of the verse's opening words
 * as fit in 60 characters, and never fewer than 20 characters of them.
 */
export function verseTitle(label: string, abbrev: string, text: string): string {
	const head = `${label} ${abbrev} – "`;
	return `${head}${clip(text, Math.max(20, TITLE - head.length - 1 - BRAND.length))}"${BRAND}`;
}

export interface Crumb {
	name: string;
	path: string;
}

/** Home › KJV › John › 3 › 16, as far as the page goes. */
export function crumbs(translation: { id: string; abbrev: string }, book?: Book, chapter?: number, verse?: number): Crumb[] {
	const out: Crumb[] = [
		{ name: 'Home', path: '/' },
		{ name: translation.abbrev, path: translationPath(translation.id) }
	];
	if (!book) return out;
	out.push({ name: book.name, path: bookPath(translation.id, book) });
	if (!chapter) return out;
	const chapterAt = `${bookPath(translation.id, book)}/${chapter}`;
	out.push({ name: String(chapter), path: chapterAt });
	if (verse) out.push({ name: String(verse), path: `${chapterAt}/${verse}` });
	return out;
}

export interface PageMeta {
	title: string;
	description: string;
	/** The canonical path, from the site's root. */
	path: string;
	crumbs: Crumb[];
	/** The link preview's image (1200 x 630), from the site's root; the site's own otherwise. */
	image?: string;
	/** The page's questions and answers, shown on it, as a FAQPage too. */
	faq?: { question: string; answer: string }[];
}

/** The page as a WebPage of the site, with its breadcrumb trail: a <script> for the <head>. */
export function jsonLd(page: PageMeta): string {
	const url = `${SITE_URL}${page.path}`;
	const site = `${SITE_URL}/#website`;
	const graph = {
		'@context': 'https://schema.org',
		'@graph': [
			{ '@type': 'WebSite', '@id': site, url: `${SITE_URL}/`, name: SITE_NAME, inLanguage: 'en' },
			{
				'@type': 'WebPage',
				'@id': url,
				url,
				name: page.title,
				description: page.description,
				inLanguage: 'en',
				isPartOf: { '@id': site },
				breadcrumb: { '@id': `${url}#breadcrumb` }
			},
			...(page.faq?.length
				? [
						{
							'@type': 'FAQPage',
							'@id': `${url}#faq`,
							url,
							isPartOf: { '@id': url },
							mainEntity: page.faq.map((f) => ({
								'@type': 'Question',
								name: f.question,
								acceptedAnswer: { '@type': 'Answer', text: f.answer }
							}))
						}
					]
				: []),
			{
				'@type': 'BreadcrumbList',
				'@id': `${url}#breadcrumb`,
				itemListElement: page.crumbs.map((c, i) => ({
					'@type': 'ListItem',
					position: i + 1,
					name: c.name,
					item: `${SITE_URL}${c.path}`
				}))
			}
		]
	};
	// A literal closing tag in the text would end the <script> block.
	return `<script type="application/ld+json">${JSON.stringify(graph).replaceAll('<', '\\u003c')}<\/script>`;
}
