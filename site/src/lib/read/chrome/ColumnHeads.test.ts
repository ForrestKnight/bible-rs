// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ReadApp } from '../app.svelte';
import type { TranslationInfo } from '../core';
import ColumnHeads from './ColumnHeads.svelte';

/** The app the heads read (getApp's context comes from Shell.svelte in the reader). */
let app: ReadApp;
vi.mock('../app.svelte', async (original) => ({
	...(await original<typeof import('../app.svelte')>()),
	getApp: () => app
}));

const translation = (id: string, name: string): TranslationInfo => ({
	id,
	abbrev: id.toUpperCase(),
	name,
	language: 'en',
	license: 'Public domain',
	copyright: '',
	year: null,
	description: null,
	redLetters: false,
	headings: true,
	size: null
});

/** Reading the KJV beside `parallel`. */
function reader(parallel: string[]): ReadApp {
	const app = new ReadApp({
		translations: [
			translation('kjv', 'King James Version'),
			translation('web', 'World English Bible'),
			translation('bsb', 'Berean Standard Bible')
		],
		translation: 'kjv',
		chapter: 0,
		content: {
			index: 0,
			book: 1,
			chapter: 1,
			status: { type: 'ready' },
			bookName: 'Genesis',
			bookTitle: 'Genesis',
			blocks: []
		},
		label: 'Genesis 1',
		short: 'Gen 1'
	});
	app.settings.parallel = parallel;
	app.settings.parallelOn = true;
	return app;
}

let heads: ReturnType<typeof mount> | undefined;

afterEach(() => {
	if (heads) unmount(heads);
	heads = undefined;
	document.body.innerHTML = '';
});

/** Opens a column head's menu and picks "Remove column". */
function removeColumn(i: number) {
	heads = mount(ColumnHeads, { target: document.body, props: { top: 0 } });
	flushSync();
	document.querySelectorAll<HTMLButtonElement>('button.pick')[i].click();
	flushSync();
	const remove = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
		(el) => el.textContent?.trim() === 'Remove column'
	);
	expect(remove, 'the menu offers "Remove column"').toBeDefined();
	remove!.click();
	flushSync();
}

describe('Remove column', () => {
	it('leaves parallel view when it removes the only extra column', () => {
		app = reader(['web']);
		expect(app.columns()).toEqual(['kjv', 'web']);
		removeColumn(1);
		expect(app.settings.parallel).toEqual([]);
		expect(app.settings.parallelOn).toBe(false);
		expect(app.columns()).toEqual(['kjv']);
		expect(document.querySelector('[role="menu"]'), 'the menu closes').toBeNull();
	});

	it('removes the column it was opened from and keeps the others', () => {
		app = reader(['web', 'bsb']);
		expect(app.columns()).toEqual(['kjv', 'web', 'bsb']);
		removeColumn(1);
		expect(app.columns()).toEqual(['kjv', 'bsb']);
		expect(app.settings.parallelOn).toBe(true);
	});
});

describe('removeParallel', () => {
	it('leaves parallel view once no extra translation is left (the translation menu unchecked)', () => {
		app = reader(['web']);
		app.dispatch({ type: 'removeParallel', id: 'web' });
		expect(app.settings.parallelOn).toBe(false);
		expect(app.columns()).toEqual(['kjv']);
	});

	it('stays in parallel view while another extra translation is left', () => {
		app = reader(['web', 'bsb']);
		app.dispatch({ type: 'removeParallel', id: 'bsb' });
		expect(app.settings.parallelOn).toBe(true);
		expect(app.columns()).toEqual(['kjv', 'web']);
	});
});
