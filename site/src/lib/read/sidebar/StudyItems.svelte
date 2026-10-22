<script lang="ts">
	import { getApp, refCmp, refEq, type StudyKind } from '../app.svelte';
	import { HIGHLIGHT_COLORS, type HighlightColor, type StudyItem, type VerseRef } from '../core';
	import { Menu, MenuItem, MenuSeparator } from '../ui';
	import ListItem from './ListItem.svelte';

	/** Items shown before "Show all". */
	const RECENT_ITEMS = 5;

	interface Props {
		kind: StudyKind;
		all: boolean;
		onshowall: () => void;
		filter: HighlightColor | null;
		onfilter: (c: HighlightColor | null) => void;
	}

	let { kind, all, onshowall, filter, onfilter }: Props = $props();

	const app = getApp();

	const HINTS: Record<StudyKind, string> = {
		bookmarks: 'No bookmarks yet. Select a verse and press B (or ⌘D).',
		highlights: 'No highlights yet. Select a verse and press 1–6.',
		notes: 'No notes yet. Select a verse and press N.'
	};

	const REMOVE: Record<StudyKind, [string, 'removeBookmark' | 'removeHighlight' | 'removeNote']> = {
		bookmarks: ['Remove bookmark', 'removeBookmark'],
		highlights: ['Remove highlight', 'removeHighlight'],
		notes: ['Delete note', 'removeNote']
	};

	/** Newest first (notes by when they changed), then in canonical order. */
	const items = $derived.by(() => {
		const list = app.studyItems(kind);
		const at = (i: StudyItem) => (kind === 'notes' ? i.updated : i.created);
		const sorted = list.toSorted((a, b) => at(b) - at(a) || refCmp(a.verse, b.verse));
		return kind === 'highlights' && filter ? sorted.filter((i) => i.highlight === filter) : sorted;
	});
	const shown = $derived(all ? items : items.slice(0, RECENT_ITEMS));
	const abbrev = $derived(app.info(app.textTranslation())?.abbrev ?? '');
	const selected = $derived(app.selection?.[0] ?? null);

	let menu = $state<{ x: number; y: number; verse: VerseRef; start: number } | null>(null);
</script>

{#if kind === 'highlights'}
	<!-- The colour filter: "All", then a swatch per colour. -->
	<div class="colours" role="radiogroup" aria-label="Highlight colour">
		<button
			type="button"
			class={['all', { on: filter === null }]}
			role="radio"
			aria-checked={filter === null}
			onclick={() => onfilter(null)}>All</button
		>
		{#each HIGHLIGHT_COLORS as c (c)}
			<button
				type="button"
				class={['swatch', { on: filter === c }]}
				style:--swatch="var(--swatch-{c})"
				role="radio"
				aria-checked={filter === c}
				aria-label="Only {app.highlightName(c)} highlights"
				title="Only {app.highlightName(c)} highlights"
				onclick={() => onfilter(filter === c ? null : c)}
			></button>
		{/each}
	</div>
{/if}

{#if items.length === 0}
	<p class="hint">{HINTS[kind]}</p>
{:else}
	{#each shown as item (`${item.verse.book}:${item.verse.chapter}:${item.verse.verse}:${item.start}`)}
		<ListItem
			reference={item.reference}
			trailing={abbrev}
			bar={kind === 'highlights' && item.highlight ? `var(--swatch-${item.highlight})` : undefined}
			note={kind === 'notes' ? item.body.replaceAll('\n', ' ') : undefined}
			snippet={item.text}
			selected={selected !== null && refEq(selected, item.verse)}
			onclick={() =>
				app.dispatch({ type: 'jump', chapter: item.chapterIndex, verse: item.verse.verse, flash: true })}
			oncontextmenu={(e) => {
				e.preventDefault();
				menu = { x: e.clientX, y: e.clientY, verse: item.verse, start: item.start };
			}}
		/>
	{/each}
	{#if shown.length < items.length}
		<button type="button" class="more" onclick={onshowall}>Show all {items.length}</button>
	{/if}
{/if}

{#if menu}
	{@const m = menu}
	<Menu anchor={new DOMRect(m.x, m.y, 0, 0)} gap={2} quiet onclose={() => (menu = null)} width={200}>
		<MenuItem
			label={REMOVE[kind][0]}
			onclick={() => {
				menu = null;
				app.dispatch(
					kind === 'highlights'
						? { type: 'removeHighlight', verse: m.verse, start: m.start }
						: { type: REMOVE[kind][1] as 'removeBookmark' | 'removeNote', verse: m.verse }
				);
			}}
		/>
		<MenuSeparator />
		<MenuItem
			label="Export as Markdown…"
			onclick={() => {
				menu = null;
				app.dispatch({ type: 'export', kind: 'markdown' });
			}}
		/>
		<MenuItem
			label="Export as JSON…"
			onclick={() => {
				menu = null;
				app.dispatch({ type: 'export', kind: 'json' });
			}}
		/>
		<MenuItem
			label="Import…"
			onclick={() => {
				menu = null;
				app.dispatch({ type: 'importStudy' });
			}}
		/>
	</Menu>
{/if}

<style>
	.colours {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 4px 0 8px 32px;
		isolation: isolate;
	}

	.all {
		height: 20px;
		padding: 0 8px;
		border-radius: 10px;
		background: var(--fill);
		color: var(--ink-2);
		font: 600 11px/20px var(--font-ui);
	}

	.all:hover:not(.on) {
		background: var(--fill-strong);
	}

	.all.on {
		background: var(--accent);
		color: var(--on-accent);
	}

	.swatch {
		position: relative;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--swatch);
		box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.18);
	}

	.swatch::before {
		content: '';
		position: absolute;
		inset: -3px;
		z-index: -1;
		border-radius: 50%;
	}

	.swatch:hover:not(.on)::before {
		background: var(--fill-strong);
	}

	.swatch.on::before {
		inset: -3.5px;
		box-shadow: inset 0 0 0 1.5px var(--ink);
	}

	.hint {
		margin: 0;
		padding: 8px 12px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.more {
		display: flex;
		align-items: center;
		width: 100%;
		height: 28px;
		padding: 0 12px;
		border-radius: var(--radius-sm);
		color: var(--accent);
		font: 500 12px/16px var(--font-ui);
		text-align: left;
	}

	.more:hover {
		background: var(--fill);
	}
</style>
