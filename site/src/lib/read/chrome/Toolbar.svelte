<!--
	The toolbar over the reading area (app.rs `toolbar`, `ds-06` in the design): back and forward
	(after the sidebar toggle when the sidebar is hidden), the location and translation pickers
	centred, the view tools and the search field at the end. Every control is a glass capsule on
	the bar's centre line; once text scrolls beneath, the bar takes the toolbar material. A compact
	bar folds the tools and the field into Search and More; on a page, a field filters the page.
-->
<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { getApp, type Page } from '../app.svelte';
	import { cmdText, shortcutText } from '../keyboard';
	import { Capsule, Menu, MenuItem, MenuSeparator, Picker, SearchField, ToolbarButton } from '../ui';
	import BookPicker from './BookPicker.svelte';
	import TextPopover from './TextPopover.svelte';
	import TranslationMenu from './TranslationMenu.svelte';

	interface Props {
		/** The window's width the sidebar takes from the main area when it's shown (0 when it
		 *  covers the reader instead). */
		sidebar: number;
		/** Whether the sidebar is shown (its header then holds the toggle). */
		sidebarShown: boolean;
	}

	let { sidebar, sidebarShown }: Props = $props();

	const app = getApp();
	const s = $derived(app.settings);
	const search = app.search;

	const LEADING_INSET = 20;
	const LEADING_INSET_SIDEBAR = 8;
	const GROUP_GAP = 8;
	const BUTTON = 36;
	const TOGGLE = 40;
	const TRAILING_INSET = 16;
	const FIELD = 176;
	const FIELD_COMPACT = 136;
	const PAGE_FIELD = 200;
	/** Below this bar width the view tools and the search field fold into one capsule. */
	const COMPACT = 760;
	const PLACEHOLDERS: Record<Page, string> = {
		reader: 'Search',
		translations: 'Filter translations',
		bookmarks: 'Search bookmarks',
		highlights: 'Search highlights',
		notes: 'Search notes',
		history: 'Search history',
		plans: 'Search plans'
	};

	let windowWidth = $state(0);
	let pickersWidth = $state(0);
	/** Shortcuts in tooltips name this platform's keys, so they wait for the browser. */
	let mounted = $state(false);
	onMount(() => (mounted = true));

	let bookOpen = $state(false);
	let translationOpen = $state(false);
	let moreOpen = $state(false);
	/** A compact bar shows the field while it's asked for (⌘F, /) until it loses focus. */
	let wantField = $state(false);

	let bookEl = $state<HTMLButtonElement>();
	let translationEl = $state<HTMLButtonElement>();
	let textEl = $state<HTMLButtonElement>();
	let moreEl = $state<HTMLButtonElement>();
	let field = $state<HTMLInputElement>();

	const reading = $derived(app.page === 'reader');
	const shown = $derived(sidebarShown && sidebar > 0);
	/** Judged at the width the bar is heading for, so a sliding sidebar switches it at once. */
	const barWidth = $derived(windowWidth - (sidebarShown ? sidebar : 0));
	const compact = $derived(windowWidth > 0 && barWidth < COMPACT);
	const showField = $derived(!compact || wantField || search.searching || search.fieldFocused);
	const fieldWidth = $derived(compact ? FIELD_COMPACT : FIELD);
	const trailingLeft = $derived(
		barWidth - TRAILING_INSET - (showField ? fieldWidth + (compact ? 0 : GROUP_GAP + 2 * BUTTON) : 2 * BUTTON)
	);

	const navLeft = $derived(shown ? LEADING_INSET_SIDEBAR : LEADING_INSET + TOGGLE + GROUP_GAP);
	const navRight = $derived(navLeft + 2 * BUTTON + 1);
	const padLeft = $derived(shown ? LEADING_INSET_SIDEBAR : LEADING_INSET);
	const centre = $derived((padLeft + barWidth - TRAILING_INSET) / 2);

	/** Centred between the bar's paddings unless that would collide with the groups at either
	 *  end; null until measured (centred by CSS), -1 when there's no room. */
	const pickersLeft = $derived.by((): number | null => {
		if (!windowWidth || !pickersWidth) return null;
		const lo = navRight + GROUP_GAP;
		const hi = trailingLeft - GROUP_GAP - pickersWidth;
		if (hi < lo) return -1;
		return Math.round(Math.max(lo, Math.min(hi, centre - pickersWidth / 2)));
	});

	const title = $derived(app.chapterName(app.here.chapter, compact));
	const abbrevs = $derived(
		s.parallelOn
			? app
					.columns()
					.map((id) => app.info(id)?.abbrev ?? id)
					.join(' · ')
			: (app.info(s.translation)?.abbrev ?? '')
	);

	const band = $derived(app.columns().length > 1 || app.resultBand);
	const material = $derived(reading ? app.underToolbar && !band : app.pageScrolled);

	const tip = (label: string, keys: string) => (mounted && keys ? `${label} (${keys})` : label);

	// ⌘F and / focus the field (showing it first in a compact bar).
	let seen = untrack(() => search.focusRequest);
	$effect(() => {
		const n = search.focusRequest;
		if (n === seen) return;
		seen = n;
		wantField = true;
		void tick().then(() => {
			field?.focus();
			field?.select();
		});
	});

	$effect(() => {
		if (app.demo.more) moreOpen = true;
	});

	function fieldKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			search.clear();
			field?.blur();
		} else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault();
			search.step(e.key === 'ArrowDown' ? 1 : -1);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			search.open();
		}
	}

	function pageKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			app.pageQuery = '';
			(e.currentTarget as HTMLInputElement).blur();
		}
	}

	function more(a: Parameters<typeof app.dispatch>[0]) {
		moreOpen = false;
		app.dispatch(a);
	}
</script>

<svelte:window bind:innerWidth={windowWidth} />

<div class="toolbar" class:material>
	<div class="slot toggle" class:away={shown} style:left="{LEADING_INSET}px">
		<Capsule>
			<ToolbarButton
				icon="sidebar"
				width={TOGGLE}
				label={tip(s.sidebarOpen ? 'Hide sidebar' : 'Show sidebar', shortcutText('sidebar', s.shortcuts))}
				onclick={() => app.dispatch({ type: 'toggleSidebar' })}
			/>
		</Capsule>
	</div>

	<div class="slot nav" style:left="{navLeft}px">
		<Capsule dividers label="History">
			<ToolbarButton
				icon="chevron-left"
				label={tip('Back', cmdText('['))}
				disabled={!(app.history.back.length > 0 || !reading)}
				onclick={() => app.dispatch({ type: 'back' })}
			/>
			<ToolbarButton
				icon="chevron-right"
				label={tip('Forward', cmdText(']'))}
				disabled={!(app.history.forward.length > 0 && reading)}
				onclick={() => app.dispatch({ type: 'forward' })}
			/>
		</Capsule>
	</div>

	{#if !reading}
		<div class="slot" style:right="{TRAILING_INSET}px">
			<SearchField
				width={PAGE_FIELD}
				placeholder={PLACEHOLDERS[app.page]}
				aria-label={PLACEHOLDERS[app.page]}
				bind:value={app.pageQuery}
				onkeydown={pageKeydown}
			/>
		</div>
	{:else}
		{#if pickersLeft !== -1}
			<div
				class="slot pickers"
				class:measuring={pickersLeft === null}
				style:left="{pickersLeft ?? centre}px"
				bind:offsetWidth={pickersWidth}
			>
				<Capsule dividers>
					<Picker
						text={title}
						label={tip('Jump to a book or chapter', shortcutText('jump', s.shortcuts))}
						padLeft={14}
						padRight={s.toolbarTranslation ? 10 : 12}
						open={bookOpen}
						bind:el={bookEl}
						onclick={() => app.core && (bookOpen = !bookOpen)}
					/>
					{#if s.toolbarTranslation}
						<Picker
							text={abbrevs}
							label={tip('Translation', cmdText('1…9'))}
							padLeft={10}
							padRight={12}
							open={translationOpen}
							bind:el={translationEl}
							onclick={() => (translationOpen = !translationOpen)}
						/>
					{/if}
				</Capsule>
			</div>
		{/if}

		{#if showField}
			{#if !compact}
				<div class="slot" style:right="{TRAILING_INSET + fieldWidth + GROUP_GAP}px">
					<Capsule label="View">
						<ToolbarButton
							icon="columns"
							label={tip('Parallel translations', shortcutText('parallel', s.shortcuts))}
							on={s.parallelOn}
							toggle
							onclick={() => app.dispatch({ type: 'toggleParallel' })}
						/>
						<ToolbarButton
							icon="text-size"
							label={tip('Text & layout', cmdText('T'))}
							open={app.textPopover}
							bind:el={textEl}
							onclick={() => app.dispatch({ type: 'textFormat' })}
						/>
					</Capsule>
				</div>
			{/if}
			<div class="slot" style:right="{TRAILING_INSET}px">
				<SearchField
					width={fieldWidth}
					aria-label="Search"
					bind:value={search.query}
					bind:input={field}
					oninput={() => search.searching && (s.sidebarOpen = true)}
					onkeydown={fieldKeydown}
					onfocus={() => (search.fieldFocused = true)}
					onblur={() => {
						search.fieldFocused = false;
						wantField = false;
					}}
				/>
			</div>
		{:else}
			<div class="slot" style:right="{TRAILING_INSET}px">
				<Capsule>
					<ToolbarButton
						icon="search-thin"
						label={tip('Search', shortcutText('search', s.shortcuts))}
						onclick={() => app.dispatch({ type: 'openSearch' })}
					/>
					<ToolbarButton icon="ellipsis" label="More" open={moreOpen} bind:el={moreEl} onclick={() => (moreOpen = !moreOpen)} />
				</Capsule>
			</div>
		{/if}
	{/if}
</div>

{#if bookOpen && bookEl && reading}
	<BookPicker anchor={bookEl} onclose={() => (bookOpen = false)} />
{/if}
{#if translationOpen && translationEl && reading}
	<TranslationMenu anchor={translationEl} onclose={() => (translationOpen = false)} />
{/if}
{#if moreOpen && moreEl && reading}
	<Menu
		anchor={moreEl}
		align="end"
		width={220}
		label="More"
		class="more-menu"
		highlight={app.demo.more && app.demo.more !== 'true' ? Number(app.demo.more) : undefined}
		onclose={() => (moreOpen = false)}
	>
		<MenuItem
			icon="columns"
			label="Parallel Translations"
			shortcut={shortcutText('parallel', s.shortcuts)}
			onclick={() => more({ type: 'toggleParallel' })}
		/>
		<MenuItem icon="text-size" label="Text & Layout" shortcut={cmdText('T')} onclick={() => more({ type: 'textFormat' })} />
		<MenuSeparator />
		<MenuItem
			icon="sidebar"
			label={s.sidebarOpen ? 'Hide Sidebar' : 'Show Sidebar'}
			shortcut={shortcutText('sidebar', s.shortcuts)}
			onclick={() => more({ type: 'toggleSidebar' })}
		/>
		<MenuItem
			icon="expand"
			label="Focus Mode"
			shortcut={shortcutText('focus', s.shortcuts)}
			onclick={() => more({ type: 'toggleFocus' })}
		/>
	</Menu>
{/if}
{#if app.textPopover && reading}
	<!-- A compact bar that's searching shows only the field: the popover hangs from that. -->
	{@const anchor = compact ? (showField ? field : moreEl) : textEl}
	{#if anchor}
		<TextPopover {anchor} align={compact ? 'end' : 'start'} onclose={() => (app.textPopover = false)} />
	{/if}
{/if}

<style>
	.toolbar {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: var(--titlebar-height);
		z-index: 4;
		pointer-events: none;
	}

	/* The toolbar material: translucent paper over the text, a hairline below. */
	.toolbar::before {
		content: '';
		position: absolute;
		inset: 0;
		background: var(--toolbar);
		box-shadow: inset 0 -1px var(--separator);
		-webkit-backdrop-filter: blur(20px) saturate(1.4);
		backdrop-filter: blur(20px) saturate(1.4);
		opacity: 0;
		transition: opacity var(--motion-fast) var(--motion-easing);
	}

	.toolbar.material::before {
		opacity: 1;
	}

	.slot {
		position: absolute;
		top: calc((var(--titlebar-height) - var(--titlebar-capsule-height)) / 2);
		pointer-events: auto;
	}

	.toggle,
	.nav {
		transition:
			left var(--motion-slow) var(--motion-easing),
			opacity var(--motion-slow) var(--motion-easing);
	}

	.toggle.away {
		opacity: 0;
		pointer-events: none;
	}

	.pickers.measuring {
		transform: translateX(-50%);
	}

	/* The Mac's rows here are 220 wide, the shortcut at the edge however close the label comes. */
	:global(.more-menu .shortcut) {
		margin-left: 10px;
	}
</style>
