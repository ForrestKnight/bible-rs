/**
 * Reader URLs: /read/<translation>/<book>/<chapter>, with the book as its name in lower case and
 * hyphens ("song-of-solomon", "1-corinthians"). The book table itself comes from the core.
 */
import type { Book } from './core';

export const bookSlug = (book: Book) => book.name.toLowerCase().replace(/\s+/g, '-');

export function chapterPath(books: Book[], translation: string, index: number, verse?: number) {
	const book = books.findLast((b) => b.firstChapter <= index) ?? books[0];
	const path = `/read/${translation}/${bookSlug(book)}/${index - book.firstChapter + 1}`;
	return verse && verse > 1 ? `${path}#v${verse}` : path;
}

/** The chapter index a URL names, if it names one. */
export function chapterFromPath(books: Book[], slug: string, chapter: string): number | null {
	const book = books.find((b) => bookSlug(b) === slug);
	const n = Number(chapter);
	if (!book || !/^[1-9]\d*$/.test(chapter) || n > book.chapters) return null;
	return book.firstChapter + n - 1;
}

export const bookOf = (books: Book[], index: number) =>
	books.findLast((b) => b.firstChapter <= index) ?? books[0];
