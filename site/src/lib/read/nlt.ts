/**
 * The NLT, read live. Once someone picks it in the translation menu, the reader fetches a chapter
 * at a time through the site's /api/nlt, which holds the key to Tyndale's NLT API. Tyndale's terms
 * allow quoting up to 500 verses and never a whole book, and ask for their credit line with the
 * text, so the page keeps one chapter, in memory only, and shows the line under it. Nothing of it
 * is prerendered or stored (TRANSLATIONS.md).
 */
import { bookOf } from './books';
import type { Book, TranslationInfo } from './core';

export const NLT_ID = 'nlt';

/** The credit line Tyndale's permission terms require with NLT text (crates/core/src/nlt_html.rs). */
export const NLT_CREDIT =
	'Scripture quotations are taken from the Holy Bible, New Living Translation, copyright ©1996, 2004, 2015 by Tyndale House Foundation. Used by permission of Tyndale House Publishers, Carol Stream, Illinois 60188. All rights reserved.';

/** Where the credit line links. */
export const NLT_SITE = 'https://www.newlivingtranslation.com';

/** The NLT in the translation menu, before any of it has loaded. */
export const NLT_INFO: TranslationInfo = {
	id: NLT_ID,
	abbrev: 'NLT',
	name: 'New Living Translation',
	language: 'en',
	license: 'Licensed text fetched as you read via the NLT API (Tyndale)',
	copyright: NLT_CREDIT,
	year: null,
	description: null,
	redLetters: true,
	headings: true,
	size: null,
	live: true
};

/** "John 3", "Song of Solomon 2": the passage /api/nlt is asked for. */
export function nltReference(books: Book[], index: number): string {
	const book = bookOf(books, index);
	return `${book.name} ${index - book.firstChapter + 1}`;
}
