import { error } from '@sveltejs/kit';
import { chapterFromPath, chapterPath } from '#lib/read/books.js';
import { chapterLabel, core, DEFAULT_CHAPTER, DEFAULT_TRANSLATION } from '#lib/read/core.js';
import { contentOptions, defaultTypography } from '#lib/read/settings.js';
import type { LayoutServerLoad } from './$types';

/**
 * Every reader page carries its chapter, typeset with the default settings, so it hydrates at once;
 * the core loads in the background for everything else. /read is John 1 until the saved place loads.
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
	const book = c.books.findLast((b) => b.firstChapter <= chapter)!;
	const text = content.blocks
		.filter((b) => b.kind.type === 'prose' || b.kind.type === 'poetry')
		.flatMap((b) => b.runs.filter((r) => !r.number).map((r) => r.text))
		.join('')
		.replace(/\s+/g, ' ')
		.trim();
	return {
		translations,
		translation,
		chapter,
		content,
		restore: !params.book,
		path: params.book ? chapterPath(c.books, translation, chapter) : '/read',
		label: chapterLabel(chapter),
		short: `${book.abbrev} ${chapter - book.firstChapter + 1}`,
		title: `${chapterLabel(chapter)} (${info.abbrev})`,
		description: text.length > 155 ? `${text.slice(0, 154).replace(/\s+\S*$/, '')}…` : text
	};
};
