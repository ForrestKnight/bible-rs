<!--
	Highlights (pages/highlights.rs): the highlighted verses as cards in two columns, each card going
	to the shorter column, its lines washed in the colour as the reader shows them. Filters by
	colour, testament and book.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { getApp } from '../app.svelte';
	import { HIGHLIGHT_COLORS, type StudyItem } from '../core';
	import { shortDate } from '../time';
	import { CapsText, Menu, MenuItem, PillTab, Segmented, SelectField } from '../ui';
	import { lines } from './measure';
	import PageEmpty from './PageEmpty.svelte';
	import PageHead from './PageHead.svelte';

	const CARD = 446;
	const GAP_X = 20;
	const GAP_Y = 16;
	const ROW = 26;

	const app = getApp();
	let color = $state<string | null>(null);
	let testament = $state(0);
	let book = $state<number | null>(null);
	let width = $state(0);
	let fonts = $state(false);
	let menu = $state<{ item: StudyItem; x: number; y: number } | null>(null);

	onMount(() => void document.fonts.load('16px "Literata Reader"').then(() => (fonts = true)));

	const all = $derived(app.studyItems('highlights'));
	const books = $derived([...new Set(all.map((h) => h.verse.book))].sort((a, b) => a - b));
	const bookName = (b: number) => app.core?.books[b]?.name ?? '';
	const subtitle = $derived.by(() => {
		if (all.length === 0) return 'Nothing highlighted yet';
		if (all.length === 1) return '1 highlight';
		return `${all.length} highlights in ${books.length} ${books.length === 1 ? 'book' : 'books'}`;
	});
	const abbrev = $derived(app.info(app.textTranslation())?.abbrev ?? '');

	const scoped = $derived.by(() => {
		const q = app.pageQuery.trim().toLowerCase();
		return all
			.filter((h) => {
				const nt = app.core?.books[h.verse.book]?.newTestament ?? false;
				return testament === 0 || (testament === 2) === nt;
			})
			.filter((h) => book === null || h.verse.book === book)
			.filter((h) => !q || h.reference.toLowerCase().includes(q) || h.text.toLowerCase().includes(q));
	});

	const layout = $derived.by(() => {
		void fonts;
		const columns = width >= 600 ? 2 : 1;
		const cardW = columns === 2 ? Math.min(CARD, (width - GAP_X) / 2) : Math.min(width, CARD + 120);
		const bottoms = Array<number>(columns).fill(0);
		const cards = scoped
			.filter((h) => color === null || h.highlight === color)
			.map((h) => {
				const text = width ? lines(h.text, '400 16px "Literata Reader"', cardW - 36, 3) : [h.text];
				const height = 58 + ROW * Math.max(1, text.length);
				const col = bottoms.indexOf(Math.min(...bottoms));
				const card = { h, text, x: col * (cardW + GAP_X), y: bottoms[col], height };
				bottoms[col] += height + GAP_Y;
				return card;
			});
		return { cardW, cards, height: Math.max(0, Math.max(...bottoms) - GAP_Y) };
	});

	const open = (h: StudyItem) => app.dispatch({ type: 'jump', chapter: h.chapterIndex, verse: h.verse.verse, flash: true });
</script>

<PageHead title="Highlights" {subtitle}>
	{#snippet actions()}
		{#if all.length}
			<SelectField
				icon="book"
				width={160}
				label="Book"
				options={[{ value: '', label: 'All books' }, ...books.map((b) => ({ value: String(b), label: bookName(b) }))]}
				value={book === null ? '' : String(book)}
				onchange={(v) => {
					book = v === '' ? null : Number(v);
					testament = 0;
				}}
			/>
			<Segmented
				layout="tight"
				label="Testament"
				options={[
					{ value: 0, label: 'All' },
					{ value: 1, label: 'OT' },
					{ value: 2, label: 'NT' }
				]}
				value={testament}
				onchange={(v) => {
					testament = v;
					book = null;
				}}
			/>
		{/if}
	{/snippet}
</PageHead>

{#if all.length === 0}
	<PageEmpty
		glyph="highlighter"
		title="Your highlights will live here"
		body="Select a verse and press 1–6 to mark it in a colour, or right-click it and pick one. 0 clears it."
		hints={[[['1–6'], 'Highlight the selected verse']]}
	/>
{:else}
	<div class="tabs" role="tablist" aria-label="Colour">
		<PillTab label="All" count={scoped.length} selected={color === null} onclick={() => (color = null)} />
		{#each HIGHLIGHT_COLORS as c (c)}
			<PillTab
				label={app.highlightName(c)}
				count={scoped.filter((h) => h.highlight === c).length}
				dot="var(--swatch-{c})"
				selected={color === c}
				onclick={() => (color = c)}
			/>
		{/each}
	</div>

	<div class="cards" bind:clientWidth={width} style:height="{layout.height}px">
		{#each layout.cards as { h, text, x, y, height } (`${h.reference}:${h.start}`)}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div
				class="card"
				role="link"
				tabindex="-1"
				style:left="{x}px"
				style:top="{y}px"
				style:width="{layout.cardW}px"
				style:height="{height}px"
				style:--wash="var(--hl-{h.highlight})"
				onclick={() => open(h)}
				oncontextmenu={(e) => {
					e.preventDefault();
					menu = { item: h, x: e.clientX, y: e.clientY };
				}}
			>
				<div class="top">
					<span class="dot" style:background="var(--swatch-{h.highlight})"></span>
					<button class="reference" onclick={(e) => (e.stopPropagation(), open(h))}>{h.reference}</button>
					<span class="abbrev">{abbrev}</span>
					<span class="date">{shortDate(h.created)}</span>
				</div>
				<div class="lines">
					{#each text as line, i (i)}
						<div class="line"><span class="run"><CapsText text={line} /></span></div>
					{/each}
				</div>
			</div>
		{/each}
	</div>
{/if}

{#if menu}
	{@const m = menu}
	<Menu anchor={new DOMRect(m.x, m.y, 0, 0)} onclose={() => (menu = null)} label="Highlight">
		<MenuItem
			label="Remove Highlight"
			onclick={() => {
				menu = null;
				app.dispatch({ type: 'removeHighlight', verse: m.item.verse, start: m.item.start });
			}}
		/>
	</Menu>
{/if}

<style>
	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		height: 28px;
		margin-bottom: 20px;
	}

	.cards {
		position: relative;
	}

	.card {
		position: absolute;
		padding: 16px;
		border-radius: 14px;
		background: var(--surface);
		box-shadow:
			0 0 0 0.5px var(--border),
			0 1px 4px rgb(0 0 0 / 0.04);
		cursor: default;
	}

	.card:hover {
		box-shadow:
			0 0 0 0.5px var(--ink-3),
			0 1px 4px rgb(0 0 0 / 0.04);
	}

	.top {
		display: flex;
		align-items: center;
		height: 16px;
	}

	.dot {
		flex: none;
		width: 10px;
		height: 10px;
		margin: 0 8px 0 0;
		border-radius: 50%;
		transform: translateY(1px);
	}

	.reference {
		color: var(--ink);
		font: 600 13px/16px var(--font-ui);
	}

	.abbrev {
		flex: 1;
		margin-left: 8px;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.date {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.lines {
		margin-top: 10px;
	}

	.line {
		height: 26px;
		padding-top: 1px;
		white-space: nowrap;
	}

	.run {
		display: inline-block;
		height: 24px;
		padding: 0 2px;
		border-radius: 3px;
		background: var(--wash);
		color: var(--ink);
		font: 400 16px/24px var(--font-reading);
	}
</style>
