<!--
	The reader's window: the sidebar, the reading area under a floating toolbar, a page in its place
	when one is open, and the popovers, sheets and toasts above (the Mac's BibleApp::ui).
-->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { dev } from '$app/env';
	import { goto } from '$app/navigation';
	import { ReadApp, setApp, now } from './app.svelte';
	import { chapterPath } from './books';
	import { chapterLabel, type ChapterContent, type TranslationInfo } from './core';
	import { NLT_ID } from './nlt';
	import Meta from './seo/Meta.svelte';
	import { chapterTitle, type Crumb } from './seo/meta';
	import { handleKey } from './keyboard';
	import { dragsFiles, fromDrop } from './imports';
	import { STUDY_KEY } from './settings';
	import { importer } from './overlays/importer.svelte';
	import Reader from './reader/Reader.svelte';
	import Toolbar from './chrome/Toolbar.svelte';
	import ColumnHeads from './chrome/ColumnHeads.svelte';
	import ResultNavigator from './chrome/ResultNavigator.svelte';
	import FocusChrome from './chrome/FocusChrome.svelte';
	import PageLinks from './chrome/PageLinks.svelte';
	import Sidebar from './sidebar/Sidebar.svelte';
	import Pages from './pages/Pages.svelte';
	import Palette from './overlays/Palette.svelte';
	import VerseMenu from './overlays/VerseMenu.svelte';
	import HighlightBar from './overlays/HighlightBar.svelte';
	import NotePopover from './overlays/NotePopover.svelte';
	import ComparePopover from './overlays/ComparePopover.svelte';
	import VersePanel from './overlays/VersePanel.svelte';
	import ShareSheet from './overlays/ShareSheet.svelte';
	import ImportSheet from './overlays/ImportSheet.svelte';
	import ShortcutsOverlay from './overlays/ShortcutsOverlay.svelte';
	import Toasts from './overlays/Toasts.svelte';
	import SettingsSheet from './settings/SettingsSheet.svelte';
	import Onboarding from './onboarding/Onboarding.svelte';

	interface Props {
		data: {
			translations: TranslationInfo[];
			translation: string;
			chapter: number;
			content: ChapterContent;
			restore: boolean;
			/** The canonical address (/read names John 1's). */
			path: string;
			label: string;
			short: string;
			title: string;
			description: string;
			crumbs: Crumb[];
		};
	}

	let { data }: Props = $props();

	const app = setApp(
		new ReadApp(
			untrack(() => ({
				translations: data.translations,
				translation: data.translation,
				chapter: data.chapter,
				content: data.content,
				label: data.label,
				short: data.short
			}))
		)
	);

	/** The toolbar's height; the bands under it (parallel heads, search results) add theirs. */
	const TOOLBAR = 52;
	const FOCUS_CHROME = 40;
	const COLUMN_HEADS = 66;
	const SEARCH_BAND = 48;
	const PAGE_TOP = 24;
	const SIDEBAR = 256;
	const SEARCH_SIDEBAR = 352;
	const SIDEBAR_INSET = 8;
	const INSPECTOR = 344;

	let booted = $state(false);
	let systemDark = $state(false);
	/** A phone's sidebar covers the reader rather than pushing it aside. */
	let phone = $state(false);

	const s = $derived(app.settings);
	const parallel = $derived(app.columns().length > 1);
	const band = $derived((parallel ? COLUMN_HEADS : 0) + (app.resultBand ? SEARCH_BAND : 0));
	const chrome = $derived(app.focus ? FOCUS_CHROME : TOOLBAR + band);
	const sidebarShown = $derived(s.sidebarOpen && !app.focus);
	const sidebarWidth = $derived(app.search?.searching ? SEARCH_SIDEBAR : SIDEBAR);
	const sidebarExtent = $derived((s.sidebarAttached ? 0 : SIDEBAR_INSET) + sidebarWidth);
	const theme = $derived(s.themeSystem ? (systemDark ? 'dark' : 'light') : s.theme);

	/** Focus mode sets the text two points larger, 36 pt lines at 22, a 680 pt measure. */
	const typo = $derived.by(() => {
		const t = s.typo;
		if (!app.focus) return { size: t.size, lineHeight: t.lineHeight, measure: t.measure };
		const size = Math.round(t.size + 2);
		return { size, lineHeight: Math.round((size * 36) / 22) / size, measure: 680 / 22 };
	});

	const title = $derived(
		app.core ? chapterTitle(chapterLabel(app.here.chapter), app.info(s.translation)?.abbrev ?? '') : data.title
	);

	onMount(() => {
		const d = document.documentElement;
		const dark = matchMedia('(prefers-color-scheme: dark)');
		systemDark = dark.matches;
		const onDark = () => (systemDark = dark.matches);
		dark.addEventListener('change', onDark);
		const narrow = matchMedia('(max-width: 700px)');
		phone = narrow.matches;
		const onNarrow = () => (phone = narrow.matches);
		narrow.addEventListener('change', onNarrow);

		app.adopt(data.restore ? null : addressed(data.translation));
		requestAnimationFrame(() => (booted = true));
		void boot();

		const tick = setInterval(() => app.tick(document.hasFocus() && !document.hidden), 1000);
		const leave = () => {
			app.finishReading();
			app.saveStudyNow();
			app.saveSettingsNow();
		};
		const onVisibility = () => document.hidden && leave();
		document.addEventListener('visibilitychange', onVisibility);
		window.addEventListener('pagehide', leave);
		const onStorage = (e: StorageEvent) => e.key === STUDY_KEY && app.studySaved(e.newValue);
		window.addEventListener('storage', onStorage);
		const backups = setInterval(() => autoBackup(), 3600_000);
		setTimeout(autoBackup, 10_000);
		if (!dev && 'serviceWorker' in navigator) {
			const sw = navigator.serviceWorker;
			// On a first visit the translations load before the worker takes over, so it hasn't kept them.
			if (!sw.controller) {
				sw.addEventListener(
					'controllerchange',
					() => {
						const ids = new Set([...app.loaded, ...app.columns()]);
						const cache = [...ids]
							.filter((id) => !app.info(id)?.imported && !app.isLive(id))
							.map((id) => `/translations/${id}.bbl`);
						sw.controller?.postMessage({ cache });
					},
					{ once: true }
				);
			}
			void sw.register('/service-worker.js', { scope: '/read' }).catch(() => {});
		}
		return () => {
			dark.removeEventListener('change', onDark);
			narrow.removeEventListener('change', onNarrow);
			clearInterval(tick);
			clearInterval(backups);
			document.removeEventListener('visibilitychange', onVisibility);
			window.removeEventListener('pagehide', leave);
			window.removeEventListener('storage', onStorage);
			leave();
			for (const k of ['read', 'theme', 'accent', 'reduceMotion', 'pending']) delete d.dataset[k];
		};
	});

	/** Starts the core, then goes to the saved place (on /read) or the verse in the address. */
	async function boot() {
		const saved = { chapter: app.settings.chapter, verse: app.settings.verse };
		const hashVerse = Number(location.hash.match(/^#v(\d+)$/)?.[1] ?? 0);
		const params = new URLSearchParams(location.search);
		const demo = dev && ['demo', 'settings', 'fixture'].some((k) => params.has(k)) ? await import('./demo') : null;
		if (dev) Object.assign(window, { readApp: app });
		demo?.configure(app, params);
		await app.start();
		if (data.restore && !demo) app.reader?.jump(saved.chapter, saved.verse > 1 ? saved.verse : undefined, false);
		else if (hashVerse > 1) app.reader?.jump(data.chapter, hashVerse, true);
		await demo?.run(app, params);
		requestAnimationFrame(() => delete document.documentElement.dataset.pending);
	}

	/** An automatic backup when one is due (daily, or weekly), as the Mac does. */
	function autoBackup() {
		if (!app.core || !app.settings.backupAuto) return;
		const last = app.backups()[0]?.at ?? 0;
		const every = app.settings.backupWeekly ? 7 * 86_400 : 86_400;
		const c = app.counts();
		if (now() - last >= every && c.highlights + c.notes + c.bookmarks > 0) app.backupNow();
	}

	// The appearance on <html>: theme, accent, reading metrics, reduced motion.
	$effect(() => {
		if (!booted && !app.core) return;
		const d = document.documentElement;
		d.dataset.theme = theme;
		d.dataset.accent = s.accent;
		d.style.setProperty('--reading-size', `${typo.size}px`);
		d.style.setProperty('--reading-line-height', String(typo.lineHeight));
		d.style.setProperty('--reading-measure', `${typo.measure}em`);
		if (s.reduceMotion) d.dataset.reduceMotion = '';
		else delete d.dataset.reduceMotion;
		const meta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
		if (meta) meta.content = getComputedStyle(d).getPropertyValue('--paper').trim() || '#ffffff';
	});

	// Settings (and the reading place) are saved shortly after they change.
	$effect(() => {
		if (!booted) return;
		JSON.stringify(app.settings);
		void app.here.chapter;
		void app.here.verse;
		const t = setTimeout(() => app.saveSettingsNow(), 800);
		return () => clearTimeout(t);
	});

	// The address follows the chapter being read. A translation imported in this browser has no
	// pages of its own: /read, which opens at the saved place. A live one's address shows the KJV's
	// page until the reader has fetched the chapter (hooks.ts).
	$effect(() => {
		if (!booted || !app.core) return;
		const path =
			app.core.bundled.includes(s.translation) || app.isLive(s.translation)
				? chapterPath(app.core.books, s.translation, app.here.chapter)
				: '/read';
		const t = setTimeout(() => {
			if (location.pathname !== path) void goto(path, { shallow: true, replace: true });
		}, 400);
		return () => clearTimeout(t);
	});

	// A file dragged onto the page opens the import sheet, whose drop zone takes it.
	function dragover(e: DragEvent) {
		if (!dragsFiles(e.dataTransfer) || app.onboarding) return;
		e.preventDefault();
		if (importer.stage !== 'choose') return;
		app.importSheet = true;
		importer.dragging = true;
	}

	async function drop(e: DragEvent) {
		if (!e.dataTransfer || !dragsFiles(e.dataTransfer)) return;
		e.preventDefault();
		importer.dragging = false;
		if (importer.stage !== 'choose' || app.onboarding) return;
		const picked = await fromDrop(e.dataTransfer);
		if (picked) await importer.choose(picked);
	}

	// A live translation follows the reader from chapter to chapter, one at a time.
	$effect(() => {
		void app.here.chapter;
		if (app.core && app.isLive(s.translation)) untrack(() => void app.ensure(s.translation));
	});

	/** The translation an address names: an NLT chapter's page carries the KJV's data (hooks.ts). */
	function addressed(translation: string): string {
		return location.pathname.startsWith(`/read/${NLT_ID}/`) ? NLT_ID : translation;
	}

	// Following a link to another chapter (a real navigation) goes there.
	let shown = { translation: untrack(() => data.translation), chapter: untrack(() => data.chapter) };
	$effect(() => {
		const { chapter, restore } = data;
		const translation = addressed(data.translation);
		untrack(() => {
			if (restore || (translation === shown.translation && chapter === shown.chapter)) return;
			shown = { translation, chapter };
			if (translation !== app.settings.translation) app.dispatch({ type: 'setTranslation', id: translation });
			if (chapter !== app.here.chapter) app.dispatch({ type: 'jump', chapter });
		});
	});
</script>

<Meta
	title={data.title}
	tabTitle={title}
	description={data.description}
	path={data.path}
	crumbs={data.crumbs}
	noindex={app.isLive(s.translation)}
/>

<svelte:head>
	<link rel="preload" href="/fonts/read/literata.woff2" as="font" type="font/woff2" crossorigin="anonymous" />
	<link rel="preload" href="/fonts/read/inter.woff2" as="font" type="font/woff2" crossorigin="anonymous" />
</svelte:head>

<svelte:window
	onkeydown={(e) => handleKey(app, e)}
	ondragover={dragover}
	ondragleave={(e) => !e.relatedTarget && (importer.dragging = false)}
	ondrop={drop}
/>

<div class="window" class:booted class:focus={app.focus}>
	{#if app.onboarding}
		<Onboarding />
		<Toasts />
	{:else}
		<aside
			class="sidebar"
			class:attached={s.sidebarAttached}
			class:shown={sidebarShown}
			style:--extent="{sidebarExtent}px"
			style:--width="{sidebarWidth}px"
			inert={!sidebarShown}
		>
			<Sidebar />
		</aside>
		<main class="main" style:--left="{sidebarShown ? sidebarExtent : 0}px">
			<div class="reading" style:right="{app.panel && app.page === 'reader' ? INSPECTOR : 0}px">
				<Reader top={chrome + PAGE_TOP} {chrome} covered={app.page !== 'reader'} />
				{#if app.page === 'reader' && band > 0}
					<div class="bands" style:height="{TOOLBAR + band}px"></div>
				{/if}
				{#if app.page === 'reader' && parallel && !app.focus}
					<ColumnHeads top={TOOLBAR} />
				{/if}
				{#if app.page === 'reader' && app.resultBand && !app.focus}
					<ResultNavigator top={TOOLBAR + (parallel ? COLUMN_HEADS : 0)} />
				{/if}
			</div>
			{#if app.page !== 'reader'}
				<Pages top={TOOLBAR} />
			{/if}
			{#if app.panel && app.page === 'reader'}
				<VersePanel top={TOOLBAR} width={INSPECTOR} />
			{/if}
			{#if app.focus}
				<FocusChrome />
			{:else}
				<Toolbar sidebar={phone ? 0 : sidebarExtent} {sidebarShown} />
			{/if}
			<Toasts />
		</main>
		{#if app.note}{#key app.note}<NotePopover />{/key}{/if}
		{#if app.compare}<ComparePopover />{/if}
		{#if app.verseMenu}<VerseMenu />{/if}
		{#if app.touch && app.selection && app.page === 'reader' && !app.verseMenu && !app.note}<HighlightBar />{/if}
		{#if app.palette}<Palette />{/if}
		{#if app.settingsTab}<SettingsSheet />{/if}
		{#if app.share}<ShareSheet />{/if}
		{#if app.importSheet}<ImportSheet />{/if}
		{#if app.shortcuts}<ShortcutsOverlay />{/if}
		<PageLinks />
	{/if}
	{#if app.coreError}
		<p class="error t-ui" role="alert">The reader couldn’t start: {app.coreError}</p>
	{/if}
</div>

<style>
	.window {
		position: fixed;
		inset: 0;
		overflow: hidden;
		background: var(--paper);
		color: var(--ink);
	}

	.window:not(.booted) :global(*) {
		transition: none !important;
	}

	/* The sidebar slides in at its full width while the main area beside it narrows. */
	.sidebar {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		width: var(--extent);
		transform: translateX(-100%);
		transition: transform var(--motion-slow) var(--motion-easing);
		z-index: 3;
	}

	.sidebar.shown {
		transform: none;
	}

	.main {
		position: absolute;
		inset: 0 0 0 var(--left);
		transition: left var(--motion-slow) var(--motion-easing);
	}

	.reading {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
	}

	/* The bands under the toolbar are solid paper (the Mac can't blur what scrolls beneath). */
	.bands {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		background: var(--paper);
		pointer-events: none;
	}

	:global(html[data-pending]) .reading {
		visibility: hidden;
	}

	/* On a phone the sidebar covers the reader instead of pushing it aside. */
	@media (max-width: 700px) {
		.main {
			left: 0;
		}
	}

	.error {
		position: fixed;
		left: 50%;
		bottom: 24px;
		transform: translateX(-50%);
		padding: 8px 14px;
		border-radius: var(--radius-sm);
		background: var(--raised);
		box-shadow: var(--shadow-popover);
		color: var(--danger);
	}
</style>
