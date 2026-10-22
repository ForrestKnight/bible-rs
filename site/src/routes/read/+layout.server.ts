import { error } from '@sveltejs/kit';
import { bookOf, chapterFromPath, chapterPath } from '#lib/read/books.js';
import { chapterLabel, core, DEFAULT_CHAPTER, DEFAULT_TRANSLATION } from '#lib/read/core.js';
import { chapterTitle, clip, crumbs } from '#lib/read/seo/meta.js';
import { contentOptions, defaultTypography } from '#lib/read/settings.js';
import type { LayoutServerLoad } from './$types';

/**
 * Every reader page carries its chapter, typeset with the default settings, so it hydrates at once;
 * the core loads in the background for everything else. /read is John 1 until the saved place loads,
 * so it names John 1 as its canonical address.
 */
export const load: LayoutServerLoad = async ({ params }) => {
	const c = await core();
	const translations = await Promise.all(c.bundled.map((id) => c.load(id)));
	const translation = params.translation ?? DEFAULT_TRANSLATION;
	if (!c.bundled.includes(translation)) error(404, 'No such translation');
	let chapter = DEFAULT_CHAPTER;
	if (params.book && params.chapter) {
		const index = chapterFromPath(c.books, params.book, params.chapter);
		if (index === null) error(404, 'No such chapter');
		chapter = index;
	}
	const content = c.chapter(translation, chapter, contentOptions(defaultTypography()));
	const info = translations.find((t) => t.id === translation)!;
	const book = bookOf(c.books, chapter);
	const n = chapter - book.firstChapter + 1;
	const text = content.blocks
		.filter((b) => b.kind.type === 'prose' || b.kind.type === 'poetry')
		.flatMap((b) => b.runs.filter((r) => !r.number).map((r) => r.text))
		.join('');
	return {
		translations,
		translation,
		chapter,
		content,
		restore: !params.book,
		path: chapterPath(c.books, translation, chapter),
		label: chapterLabel(chapter),
		short: `${book.abbrev} ${n}`,
		title: chapterTitle(chapterLabel(chapter), info.abbrev),
		description: clip(text, 155),
		crumbs: crumbs(info, book, n)
	};
};
