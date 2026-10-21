/**
 * Development only: puts the reader in a given state from the address, with the steps the Mac app
 * takes from BIBLE_RS_DEMO (app.rs `demo_step`), so the web and Mac renders can be compared.
 *
 *   /read/kjv/john/1?fixture&settings={"sidebarOpen":true}&demo=jump:jn 3:16;select:jn 3:16
 *
 * `fixture` imports the study the Mac reference renders use; `settings` merges into the settings.
 * Steps the app handles itself are below; any other `cmd:arg` lands in `app.demo[cmd]` for the
 * component it concerns (`study`, `scope`, `more`, `reveal`, `pick`, `plans-tab`, `plan-day`,
 * `note-select`, `onboarding`, `record`, `palette`, `menu`). When it's done, <html> gets
 * `data-demo-done`. The import steps are the Mac's: `import` (`import:fixture` chooses the fixture
 * folder), then `import-drag`, `import-progress:<books>` or `import-review:<error>|<key tried>`.
 */
import type { ReadApp } from './app.svelte';
import { parseReference, type VerseRef } from './core';
import { importer } from './overlays/importer.svelte';
import { merge, type Settings } from './settings';

/** The import fixture the Mac renders use (a folder: John, and Tobit, which is left out). */
const SOURCE = {
	label: 'MyBible',
	files: [
		{
			path: '43JHN.usfm',
			file: new Blob([
				'\\id JHN\n\\h John\n\\c 1\n\\p\n\\v 1 In the beginning was the Word, and the Word was with God, and the Word was God.\n\\v 2 He was with God in the beginning.\n\\c 3\n\\p\n\\v 16 For God so loved the world that He gave His one and only Son, that everyone who believes in Him shall not perish but have eternal life.\n'
			])
		},
		{ path: '68TOB.usfm', file: new Blob(['\\id TOB\n\\c 1\n\\p\n\\v 1 The book of the words of Tobit.\n']) }
	]
};

const FIXTURE = JSON.stringify({
	app: 'bible-rs',
	format: 1,
	exported_at: '2026-10-14T12:00:00Z',
	translation: 'KJV',
	highlights: [
		['John 1:1', 'JHN', 1, 1, 'yellow'],
		['John 1:4', 'JHN', 1, 4, 'green'],
		['John 1:5', 'JHN', 1, 5, 'green'],
		['John 1:14', 'JHN', 1, 14, 'blue'],
		['John 3:16', 'JHN', 3, 16, 'yellow'],
		['Psalm 23:1', 'PSA', 23, 1, 'green'],
		['Romans 8:28', 'ROM', 8, 28, 'pink'],
		['Romans 8:38', 'ROM', 8, 38, 'purple'],
		['Romans 8:39', 'ROM', 8, 39, 'purple']
	].map(([reference, book, chapter, verse, color]) => ({
		reference,
		book,
		chapter,
		verse,
		color,
		created: '2026-10-14T12:00:00Z'
	})),
	notes: [
		{
			reference: 'John 1:14',
			book: 'JHN',
			chapter: 1,
			verse: 14,
			note: 'The Word became flesh — compare Philippians 2:6–8 and Hebrews 1:3.',
			created: '2026-10-14T12:00:00Z',
			updated: '2026-10-14T12:00:00Z'
		},
		{
			reference: 'Romans 8:28',
			book: 'ROM',
			chapter: 8,
			verse: 28,
			note: 'Not that all things are good, but that God works all things together for good.\n\nSee Genesis 50:20.',
			created: '2026-10-14T12:00:00Z',
			updated: '2026-10-14T12:00:00Z'
		}
	],
	bookmarks: [
		['John 1:1', 'JHN', 1, 1, 'Prologue'],
		['Psalm 23:1', 'PSA', 23, 1, 'The Shepherd'],
		['Romans 8:28', 'ROM', 8, 28, '']
	].map(([reference, book, chapter, verse, label]) => ({
		reference,
		book,
		chapter,
		verse,
		label,
		created: '2026-10-14T12:00:00Z'
	}))
});

/** Before the core starts: the settings, as the Mac renders set them (onboarded unless asked). */
export function configure(app: ReadApp, params: URLSearchParams) {
	const extra = JSON.parse(params.get('settings') ?? '{}');
	app.settings = merge<Settings>(JSON.parse(JSON.stringify(app.settings)), { onboarded: true, ...extra });
	app.onboarding = !app.settings.onboarded;
}

/** After the core starts: the fixture, then the steps in order. */
export async function run(app: ReadApp, params: URLSearchParams) {
	if (params.has('fixture') && app.counts().highlights === 0) {
		app.importStudyText(FIXTURE);
		app.toasts = [];
	}
	for (const step of (params.get('demo') ?? '').split(';').filter(Boolean)) {
		await frames(2);
		await apply(app, step);
	}
	await frames(4);
	document.documentElement.dataset.demoDone = '';
}

async function apply(app: ReadApp, step: string) {
	const at = step.indexOf(':');
	const cmd = at < 0 ? step : step.slice(0, at);
	const arg = at < 0 ? '' : step.slice(at + 1);
	const ref = parseReference(arg.split('@')[0]);
	const verse: VerseRef | null = ref?.verse ? { book: ref.book, chapter: ref.chapter, verse: ref.verse } : null;
	switch (cmd) {
		case 'jump':
			if (ref) app.dispatch({ type: 'jump', chapter: ref.chapterIndex, verse: ref.verse ?? undefined, flash: !!ref.verse });
			await frames(6);
			break;
		case 'select':
			app.setSelection(verse);
			break;
		case 'palette':
			app.demo.palette = arg;
			app.dispatch({ type: 'openPalette' });
			break;
		case 'search':
			app.dispatch({ type: 'openSearch', query: arg });
			await until(() => app.search.results !== null);
			break;
		case 'step':
			app.dispatch({ type: 'searchStep', delta: Number(arg) || 1 });
			break;
		case 'note':
			if (verse) app.dispatch({ type: 'editNote', verse });
			break;
		case 'compare':
			app.setSelection(verse);
			app.dispatch({ type: 'compare' });
			break;
		case 'study':
			app.demo.study = arg;
			app.dispatch({ type: 'sidebarTab', tab: 'study' });
			break;
		case 'menu': {
			const [x, y] = (arg.split('@')[1] ?? '600,352').split(',').map(Number);
			app.setSelection(verse);
			app.demo.menu = 'first';
			app.dispatch({ type: 'openVerseMenu', x, y });
			break;
		}
		case 'share':
			app.dispatch({ type: 'shareImage' });
			break;
		case 'focus':
			app.dispatch({ type: 'toggleFocus' });
			break;
		case 'shortcuts':
			app.shortcuts = true;
			break;
		case 'page':
			app.dispatch({ type: 'openPage', page: arg as ReadApp['page'] });
			break;
		case 'settings':
			app.dispatch({ type: 'openSettings', tab: (arg || 'reading') as NonNullable<ReadApp['settingsTab']> });
			break;
		case 'import':
			app.importSheet = true;
			if (arg) await importer.choose(SOURCE);
			break;
		case 'import-drag':
			importer.dragging = true;
			break;
		case 'import-progress':
			importer.demoConverting(Number(arg) || 47);
			break;
		case 'import-review': {
			const [error, key] = arg.split('|');
			await importer.demoReview(app, error || null, key || null);
			break;
		}
		case 'onboarding':
			app.demo.onboarding = arg || '1';
			app.onboarding = true;
			break;
		default:
			app.demo[cmd] = arg || 'true';
	}
}

const frames = (n: number) =>
	new Promise<void>((resolve) => {
		const next = (left: number) => (left ? requestAnimationFrame(() => next(left - 1)) : resolve());
		next(n);
	});

/** Waits up to a second for a condition (a component registering itself). */
async function until(ok: () => boolean) {
	for (let i = 0; i < 60 && !ok(); i++) await frames(1);
	return ok();
}
