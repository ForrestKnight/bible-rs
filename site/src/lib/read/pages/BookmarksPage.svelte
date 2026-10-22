<!--
	Bookmarks (pages/bookmarks.rs): the bookmarked verses in canonical order under their books, or
	newest first, each with its text; hovering a row offers Go and Remove in place of its date.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import type { StudyItem } from '../core';
	import { shortDate } from '../time';
	import { Button, CapsText, Icon, RoundButton, Segmented } from '../ui';
	import PageEmpty from './PageEmpty.svelte';
	import PageHead from './PageHead.svelte';

	const app = getApp();
	let recent = $state(false);

	const all = $derived(app.studyItems('bookmarks'));
	const subtitle = $derived.by(() => {
		const books = new Set(all.map((b) => b.verse.book)).size;
		if (all.length === 0) return 'Nothing bookmarked yet';
		if (all.length === 1) return '1 verse';
		return `${all.length} verses in ${books} ${books === 1 ? 'book' : 'books'}`;
	});
	const abbrev = $derived(app.info(app.textTranslation())?.abbrev ?? '');

	const rows = $derived.by(() => {
		const q = app.pageQuery.trim().toLowerCase();
		const out = all.filter(
			(b) => !q || b.reference.toLowerCase().includes(q) || b.text.toLowerCase().includes(q)
		);
		if (recent) out.sort((a, b) => b.created - a.created || a.chapterIndex - b.chapterIndex || a.verse.verse - b.verse.verse);
		return out;
	});

	const open = (b: StudyItem) =>
		app.dispatch({ type: 'jump', chapter: b.chapterIndex, verse: b.verse.verse, flash: true });
</script>

<PageHead title="Bookmarks" {subtitle}>
	{#snippet actions()}
		{#if all.length}
			<Segmented
				layout="tight"
				label="Order"
				options={[
					{ value: 0, label: 'Canonical' },
					{ value: 1, label: 'Recent' }
				]}
				value={recent ? 1 : 0}
				onchange={(v) => (recent = v === 1)}
			/>
			<Button icon="upload" label="Export" onclick={() => app.exportStudy('markdown')} />
		{/if}
	{/snippet}
</PageHead>

{#if all.length === 0}
	<PageEmpty
		glyph="bookmark"
		title="Your bookmarks will live here"
		body="Select a verse and press B, or right-click it and choose Bookmark. Bookmarks stay in this browser and export with your notes."
		hints={[[['B'], 'Bookmark the selected verse']]}
	/>
{:else}
	<div class="rows">
		{#each rows as b, i (b.reference)}
			{#if !recent && b.verse.book !== rows[i - 1]?.verse.book}
				<h2 class="group-label">{app.core?.books[b.verse.book]?.name}</h2>
			{/if}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="row" role="link" tabindex="-1" onclick={() => open(b)}>
				<span class="glyph"><Icon name="bookmark-fill" size={16} heavy /></span>
				<div class="main">
					<div class="ref">
						<button class="reference" onclick={(e) => (e.stopPropagation(), open(b))}>{b.reference}</button>
						<span class="abbrev">{abbrev}</span>
					</div>
					<p class="text"><CapsText text={b.text} /></p>
				</div>
				<span class="date">{shortDate(b.created)}</span>
				<span class="tools">
					<RoundButton icon="arrow-right" label="Go to {b.reference}" onclick={(e) => (e.stopPropagation(), open(b))} />
					<RoundButton
						icon="trash"
						label="Remove bookmark"
						onclick={(e) => (e.stopPropagation(), app.dispatch({ type: 'removeBookmark', verse: b.verse }))}
					/>
				</span>
			</div>
		{/each}
	</div>
{/if}

<style>
	.group-label {
		height: 32px;
		padding-top: 12px;
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.row {
		position: relative;
		display: flex;
		padding: 12px 16px 12px 44px;
		border-radius: 12px;
		cursor: default;
	}

	.row::after {
		position: absolute;
		right: 0;
		bottom: 0;
		left: 0;
		height: 1px;
		background: var(--separator);
		content: '';
	}

	.row:hover {
		background: var(--fill);
	}

	.row:hover::after {
		display: none;
	}

	.row:active {
		background: var(--fill-strong);
	}

	.glyph {
		position: absolute;
		top: 18px;
		left: 14px;
		display: flex;
		color: var(--accent);
	}

	.main {
		flex: 1;
		min-width: 0;
	}

	.ref {
		display: flex;
		align-items: baseline;
		gap: 8px;
		height: 22px;
	}

	.reference {
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.abbrev {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	/* The Mac sets each 28 pt row's text in a 24 pt box at its top. */
	.text {
		display: -webkit-box;
		max-width: min(620px, calc(100% - 80px));
		margin: -2px 0 2px;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		color: var(--ink);
		font: 400 17px/28px var(--font-reading);
	}

	.date {
		position: absolute;
		top: 16px;
		right: 16px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.tools {
		position: absolute;
		top: 12px;
		right: 16px;
		display: none;
		gap: 2px;
	}

	.row:hover .tools,
	.row:focus-within .tools {
		display: flex;
	}

	.row:hover .date,
	.row:focus-within .date {
		display: none;
	}
</style>
