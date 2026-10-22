/**
 * Reader URLs: /read/<translation>/<book>/<chapter>[/<verse>], with the book as its name in lower
 * case and hyphens ("song-of-solomon", "1-corinthians"); /read/<translation>/<book> and
 * /read/<translation> are the book's and the translation's index pages.
 *
 * The book table is the core's (wasm `books()`), repeated here so a page can list, name and link
 * the books before the core has loaded. core.ts checks they agree whenever it starts on the
 * server, so a build fails if they ever drift apart.
 */
import type { Book } from './core';

/** Name, abbreviation, chapters, in canonical order; Matthew (39) starts the New Testament. */
const TABLE: [string, string, number][] = [
	['Genesis', 'Gen', 50],
	['Exodus', 'Exod', 40],
	['Leviticus', 'Lev', 27],
	['Numbers', 'Num', 36],
	['Deuteronomy', 'Deut', 34],
	['Joshua', 'Josh', 24],
	['Judges', 'Judg', 21],
	['Ruth', 'Ruth', 4],
	['1 Samuel', '1 Sam', 31],
	['2 Samuel', '2 Sam', 24],
	['1 Kings', '1 Kgs', 22],
	['2 Kings', '2 Kgs', 25],
	['1 Chronicles', '1 Chr', 29],
	['2 Chronicles', '2 Chr', 36],
	['Ezra', 'Ezra', 10],
	['Nehemiah', 'Neh', 13],
	['Esther', 'Esth', 10],
	['Job', 'Job', 42],
	['Psalms', 'Ps', 150],
	['Proverbs', 'Prov', 31],
	['Ecclesiastes', 'Eccl', 12],
	['Song of Solomon', 'Song', 8],
	['Isaiah', 'Isa', 66],
	['Jeremiah', 'Jer', 52],
	['Lamentations', 'Lam', 5],
	['Ezekiel', 'Ezek', 48],
	['Daniel', 'Dan', 12],
	['Hosea', 'Hos', 14],
	['Joel', 'Joel', 3],
	['Amos', 'Amos', 9],
	['Obadiah', 'Obad', 1],
	['Jonah', 'Jonah', 4],
	['Micah', 'Mic', 7],
	['Nahum', 'Nah', 3],
	['Habakkuk', 'Hab', 3],
	['Zephaniah', 'Zeph', 3],
	['Haggai', 'Hag', 2],
	['Zechariah', 'Zech', 14],
	['Malachi', 'Mal', 4],
	['Matthew', 'Matt', 28],
	['Mark', 'Mark', 16],
	['Luke', 'Luke', 24],
	['John', 'John', 21],
	['Acts', 'Acts', 28],
	['Romans', 'Rom', 16],
	['1 Corinthians', '1 Cor', 16],
	['2 Corinthians', '2 Cor', 13],
	['Galatians', 'Gal', 6],
	['Ephesians', 'Eph', 6],
	['Philippians', 'Phil', 4],
	['Colossians', 'Col', 4],
	['1 Thessalonians', '1 Thess', 5],
	['2 Thessalonians', '2 Thess', 3],
	['1 Timothy', '1 Tim', 6],
	['2 Timothy', '2 Tim', 4],
	['Titus', 'Titus', 3],
	['Philemon', 'Phlm', 1],
	['Hebrews', 'Heb', 13],
	['James', 'Jas', 5],
	['1 Peter', '1 Pet', 5],
	['2 Peter', '2 Pet', 3],
	['1 John', '1 John', 5],
	['2 John', '2 John', 1],
	['3 John', '3 John', 1],
	['Jude', 'Jude', 1],
	['Revelation', 'Rev', 22]
];

const NEW_TESTAMENT = 39;

export const BOOKS: Book[] = TABLE.reduce<Book[]>((out, [name, abbrev, chapters], index) => {
	const prev = out[index - 1];
	const firstChapter = prev ? prev.firstChapter + prev.chapters : 0;
	out.push({ index, name, abbrev, chapters, firstChapter, newTestament: index >= NEW_TESTAMENT });
	return out;
}, []);

export const bookSlug = (book: Book) => book.name.toLowerCase().replace(/\s+/g, '-');

export const bookOf = (books: Book[], index: number) =>
	books.findLast((b) => b.firstChapter <= index) ?? books[0];

/** "John 3", "Psalm 23": the core's `chapterLabel`, for before the core has loaded. */
export function chapterName(books: Book[], index: number) {
	const book = bookOf(books, index);
	return `${book.name === 'Psalms' ? 'Psalm' : book.name} ${index - book.firstChapter + 1}`;
}

export const translationPath = (translation: string) => `/read/${translation}`;

export const bookPath = (translation: string, book: Book) => `/read/${translation}/${bookSlug(book)}`;

export function chapterPath(books: Book[], translation: string, index: number, verse?: number) {
	const book = bookOf(books, index);
	const path = `${bookPath(translation, book)}/${index - book.firstChapter + 1}`;
	return verse && verse > 1 ? `${path}#v${verse}` : path;
}

/** A verse's own page (only some translations have them: see seo.ts `VERSE_PAGES`). */
export const versePath = (books: Book[], translation: string, index: number, verse: number) =>
	`${chapterPath(books, translation, index)}/${verse}`;

export const bookFromSlug = (books: Book[], slug: string) => books.find((b) => bookSlug(b) === slug) ?? null;

/** The chapter index a URL names, if it names one. */
export function chapterFromPath(books: Book[], slug: string, chapter: string): number | null {
	const book = bookFromSlug(books, slug);
	const n = Number(chapter);
	if (!book || !/^[1-9]\d*$/.test(chapter) || n > book.chapters) return null;
	return book.firstChapter + n - 1;
}
