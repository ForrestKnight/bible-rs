<!--
	The pages that take the reader's place (pages/mod.rs): the translation library and the study
	collections. Each scrolls under the toolbar, 40 pt from the sides and 24 pt below the toolbar;
	the notes split into a list and an editor that scroll on their own once there are any.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Button } from '../ui';
	import BookmarksPage from './BookmarksPage.svelte';
	import HighlightsPage from './HighlightsPage.svelte';
	import HistoryPage from './HistoryPage.svelte';
	import NotesPage from './NotesPage.svelte';
	import { notesPage } from './notes.svelte';
	import PageEmpty from './PageEmpty.svelte';
	import PageHead from './PageHead.svelte';
	import PlansPage from './PlansPage.svelte';
	import TranslationsPage from './TranslationsPage.svelte';

	let { top }: { top: number } = $props();

	const app = getApp();
	let scroller = $state<HTMLElement>();

	const panes = $derived(app.page === 'notes' && (app.counts().notes > 0 || notesPage.selected !== null));

	// A page opens at its top, with the toolbar clear.
	$effect(() => {
		void app.page;
		scroller?.scrollTo({ top: 0 });
		app.pageScrolled = false;
	});
</script>

{#if panes}
	<NotesPage {top} />
{:else}
	<div
		class="pages"
		bind:this={scroller}
		style:--toolbar="{top}px"
		onscroll={(e) => (app.pageScrolled = e.currentTarget.scrollTop > 0.5)}
	>
		<div class="content">
			{#if app.page === 'translations'}<TranslationsPage />
			{:else if app.page === 'bookmarks'}<BookmarksPage />
			{:else if app.page === 'highlights'}<HighlightsPage />
			{:else if app.page === 'history'}<HistoryPage />
			{:else if app.page === 'plans'}<PlansPage />
			{:else if app.page === 'notes'}
				<PageHead title="Notes" subtitle="Nothing written yet" />
				<PageEmpty
					glyph="note"
					title="Your notes will live here"
					body="Select a verse and press N, or right-click it and choose Add Note. Notes stay in this browser and export to Markdown."
					hints={[
						[['N'], 'New note on the selected verse'],
						[['⌘', 'E'], 'Export as Markdown']
					]}
					width={344}
				>
					{#snippet more()}
						<Button kind="ghost" label="Import a backup" onclick={() => app.dispatch({ type: 'importStudy' })} />
					{/snippet}
				</PageEmpty>
			{/if}
		</div>
	</div>
{/if}

<style>
	.pages {
		position: absolute;
		inset: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
		background: var(--paper);
		color: var(--ink);
		font: var(--type-ui);
	}

	.content {
		position: relative;
		min-height: 100%;
		padding: calc(var(--toolbar) + 24px) 40px 40px;
	}
</style>
