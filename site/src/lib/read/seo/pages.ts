/**
 * The reader's prerendered pages, listed once for both the routes' `entries` and the sitemaps, so
 * a sitemap never names a page the build didn't make.
 */
import { bookPath, chapterPath, versePath } from '../books';
import type { Book, Core } from '../core';
import { VERSE_PAGES } from './meta';

/** Where a link to a verse goes: its own page, or its place in the chapter (#v16) in a translation without verse pages. */
export const verseHref = (books: Book[], translation: string, index: number, verse: number) =>
	VERSE_PAGES.includes(translation)
		? versePath(books, translation, index, verse)
		: chapterPath(books, translation, index, verse);

export interface ChapterPage {
	translation: string;
	book: Book;
	/** From 1. */
	chapter: number;
}

export interface VersePage extends ChapterPage {
	verse: number;
}

export const chapterPagePath = (p: ChapterPage) => `${bookPath(p.translation, p.book)}/${p.chapter}`;
export const versePagePath = (p: VersePage) => `${chapterPagePath(p)}/${p.verse}`;

/** Every book of every bundled translation (/read/kjv/john). */
export const bookPages = (c: Core) => c.bundled.flatMap((translation) => c.books.map((book) => ({ translation, book })));

/** Every chapter of every bundled translation (/read/kjv/john/3). */
export const chapterPages = (c: Core): ChapterPage[] =>
	bookPages(c).flatMap(({ translation, book }) =>
		Array.from({ length: book.chapters }, (_, i) => ({ translation, book, chapter: i + 1 }))
	);

/** The verses with pages of their own: every verse a VERSE_PAGES translation has (/read/kjv/john/3/16). */
export async function versePages(c: Core): Promise<VersePage[]> {
	const out: VersePage[] = [];
	for (const translation of VERSE_PAGES.filter((id) => c.bundled.includes(id))) {
		await c.load(translation);
		for (const book of c.books) {
			for (let chapter = 1; chapter <= book.chapters; chapter++) {
				for (const v of c.verses(translation, book.firstChapter + chapter - 1)) {
					out.push({ translation, book, chapter, verse: v.number });
				}
			}
		}
	}
	return out;
}
