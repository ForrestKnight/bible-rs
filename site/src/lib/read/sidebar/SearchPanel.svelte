<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { getApp } from '../app.svelte';
	import type { SearchScope } from '../core';
	import { FIRST, group, snippet, type Snippet as Marked } from '../search.svelte';
	import { Button, Checkbox, EmptyState, Icon, Menu, MenuItem, MenuSeparator, Segmented } from '../ui';
	import ListItem from './ListItem.svelte';

	const app = getApp();
	const search = app.search;

	/** Space either side of the controls and of the list, and inside an item. */
	const CONTROLS_INSET = 8;
	const LIST_INSET = 8;
	const ITEM_PAD_X = 12;
	const HEADER = 28;
	/** Rendered beyond the viewport, each way. */
	const OVERSCAN = 400;

	type Entry =
		| { kind: 'header'; name: string; count: number; y: number; h: number }
		| { kind: 'item'; i: number; rows: number; y: number; h: number };

	let list = $state<HTMLElement>();
	let scrollTop = $state(0);
	let viewport = $state(0);
	let width = $state(352);
	let fontsReady = $state(false);
	let scopeButton = $state<HTMLElement>();
	let orderButton = $state<HTMLElement>();
	let scopeMenu = $state(false);
	let orderMenu = $state(false);

	const results = $derived(search.results);
	const books = $derived(app.core?.books ?? []);
	const here = $derived(app.hereRef().book);
	const abbrev = $derived(app.info(app.textTranslation())?.abbrev ?? '');
	const scope = $derived(search.scope);
	const segment = $derived(
		typeof scope === 'number' ? (books[scope]?.newTestament ? 'nt' : 'ot') : (scope as 'all' | 'ot' | 'nt')
	);

	onMount(() => {
		void document.fonts.load('14px "Literata Reader"').then(() => (fontsReady = true));
	});

	let canvas: CanvasRenderingContext2D | null = null;
	/** A snippet's width on one line, from its start (after "…" when it starts mid-verse). */
	function measure(s: Marked): number {
		canvas ??= document.createElement('canvas').getContext('2d');
		if (!canvas) return 0;
		canvas.font = '400 14px "Literata Reader"';
		return canvas.measureText(s.runs.map((r) => r.text).join('')).width + (s.cut ? 12 : 0);
	}

	/** Headers 28 pt, hits 60 (one snippet line) or 80 (two), at precomputed positions so the list
	 *  can be virtualized. Hits past the first page aren't in yet; they get two lines. */
	const entries = $derived.by((): Entry[] => {
		if (!results || !fontsReady) return [];
		// Measured within the controls' margins, as the Mac does: a snippet that nearly fills its
		// line gets two.
		const room = width - 2 * (CONTROLS_INSET + LIST_INSET + ITEM_PAD_X);
		const out: Entry[] = [];
		let y = 0;
		let i = 0;
		for (const b of results.books) {
			out.push({ kind: 'header', name: b.name, count: b.count, y, h: HEADER });
			y += HEADER;
			for (let k = 0; k < b.count; k++, i++) {
				const hit = i < FIRST ? results.hits[i] : null;
				const rows = hit && measure(snippet(hit)) <= room ? 1 : 2;
				const h = 40 + 20 * rows;
				out.push({ kind: 'item', i, rows, y, h });
				y += h;
			}
		}
		return out;
	});
	const total = $derived(entries.length ? entries[entries.length - 1].y + entries[entries.length - 1].h : 0);

	const visible = $derived.by(() => {
		const top = scrollTop - OVERSCAN;
		const bottom = scrollTop + viewport + OVERSCAN;
		let lo = 0;
		let hi = entries.length;
		while (lo < hi) {
			const mid = (lo + hi) >> 1;
			if (entries[mid].y + entries[mid].h < top) lo = mid + 1;
			else hi = mid;
		}
		const out: Entry[] = [];
		for (let k = lo; k < entries.length && entries[k].y <= bottom; k++) out.push(entries[k]);
		return out;
	});

	// A new search starts at the top of its results.
	$effect(() => {
		void search.key;
		untrack(() => {
			if (list) list.scrollTop = 0;
			scrollTop = 0;
		});
	});

	// Stepping through results keeps the selected one in view, a third of the way down.
	$effect(() => {
		if (!search.scrollRequest) return;
		untrack(() => {
			void tick().then(() => {
				const e = entries.find((x) => x.kind === 'item' && x.i === search.index);
				if (!e || !list) return;
				const avail = list.clientHeight;
				if (e.y < list.scrollTop || e.y + e.h > list.scrollTop + avail) {
					list.scrollTop = Math.max(0, e.y - avail / 3);
				}
			});
		});
	});

	/** For a search that found nothing: a spelling that finds verses, and its first verse. */
	const suggestion = $derived.by(() => {
		const s = results?.total === 0 ? results.suggestion : null;
		if (!s || !app.core) return null;
		const first = app.core.search(app.textTranslation(), s.query, scope, app.settings.wholeWords, null, 0, 1)
			.hits[0];
		const reference = first?.reference ?? '';
		const detail = s.total === 1 ? `1 verse · ${reference}` : `${group(s.total)} verses · first ${reference}`;
		return { query: s.query, detail };
	});

	const filtered = $derived(scope !== 'all' || app.settings.wholeWords);
	const scoped = $derived(scope !== 'all');

	function setScope(s: SearchScope) {
		search.scope = s;
		scopeMenu = false;
	}

	function summary(n: number, b: number) {
		return `${group(n)} ${n === 1 ? 'verse' : 'verses'} in ${b} ${b === 1 ? 'book' : 'books'}`;
	}
</script>

{#snippet widen()}
	{#if scoped}
		<Button kind="secondary" label="Search Whole Bible" onclick={() => (search.scope = 'all')} />
	{/if}
	<Button
		kind="ghost"
		label="Clear filters"
		onclick={() => {
			search.scope = 'all';
			app.settings.wholeWords = false;
		}}
	/>
{/snippet}

{#snippet marked(s: Marked)}{#if s.cut}…{/if}{#each s.runs as r, k (k)}{#if r.hit}<mark>{r.text}</mark
			>{:else}{r.text}{/if}{/each}{/snippet}

<div class="panel">
	<div class="controls">
		<Segmented
			label="Search in"
			options={[
				{ value: 'all', label: 'Bible' },
				{ value: 'ot', label: 'Old Testament' },
				{ value: 'nt', label: 'New Testament' }
			]}
			value={segment}
			onchange={(v) => (search.scope = v)}
		/>
		<div class="filters">
			<button
				bind:this={scopeButton}
				type="button"
				class={['select', { open: scopeMenu }]}
				aria-haspopup="menu"
				aria-expanded={scopeMenu}
				onclick={() => (scopeMenu = !scopeMenu)}
			>
				<Icon name="book" size={16} heavy class="lead" />
				<span class="value">{typeof scope === 'number' ? books[scope]?.name : 'All books'}</span>
				<Icon name="chevrons-up-down" size={16} heavy class="chevrons" />
			</button>
			<Checkbox label="Whole words" bind:checked={app.settings.wholeWords} />
		</div>
		{#if results && results.total === 0}
			<p class="count">No verses match</p>
		{:else if results}
			<div class="count">
				<span title="{search.elapsed.toFixed(2)} ms · {abbrev}">{summary(results.total, results.books.length)}</span>
				<button
					bind:this={orderButton}
					type="button"
					class="order"
					aria-haspopup="menu"
					aria-expanded={orderMenu}
					onclick={() => (orderMenu = !orderMenu)}
				>
					{search.currentFirst ? 'This book first' : 'Canonical'}
					<Icon name="chevron-down" size={12} />
				</button>
			</div>
		{/if}
	</div>

	{#if results && results.total === 0}
		<div class="none">
			<EmptyState
				glyph="search-thin"
				title="No results for “{search.query.trim()}”"
				body={search.query.trim().split(/\s+/).length > 1
					? 'Every word has to appear in the same verse.\nTry fewer words or search the whole Bible.'
					: 'Check the spelling, or try another word\nor the whole Bible.'}
				buttons={filtered ? widen : undefined}
			>
				{#snippet after()}
					{#if suggestion}
						<button type="button" class="suggestion" onclick={() => (search.query = suggestion.query)}>
							<span class="did">Did you mean</span>
							<span class="line">
								<span class="q">“{suggestion.query}”</span>
								<span class="detail">{suggestion.detail}</span>
							</span>
						</button>
					{/if}
				{/snippet}
			</EmptyState>
		</div>
	{:else if results}
		<div
			class="results"
			bind:this={list}
			bind:clientHeight={viewport}
			bind:clientWidth={width}
			onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
		>
			<div class="track" style:height="{total + 8}px">
				{#each visible as e (e.kind === 'item' ? e.i : `h${e.y}`)}
					<div class="entry" style:top="{e.y}px" style:height="{e.h}px">
						{#if e.kind === 'header'}
							<div class="header">
								<span class="book">{e.name}</span>
								<span class="n">{e.count}</span>
							</div>
						{:else}
							{@const hit = search.hit(e.i)}
							{#if hit}
								{@const s = snippet(hit)}
								<ListItem
									reference={hit.reference}
									trailing={abbrev}
									lines={e.rows}
									selected={e.i === search.index}
									onclick={() => search.choose(e.i)}
								>
									{#snippet text()}{@render marked(s)}{/snippet}
								</ListItem>
							{/if}
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>

{#if scopeMenu && scopeButton}
	{@const testament = scope === 'ot' ? false : scope === 'nt' ? true : null}
	<Menu anchor={scopeButton} quiet width={200} onclose={() => (scopeMenu = false)} label="Books to search">
		<MenuItem
			label="All books"
			selected={typeof scope !== 'number'}
			onclick={() => setScope(testament === null ? 'all' : testament ? 'nt' : 'ot')}
		/>
		{#if books[here]}
			<MenuItem label="{books[here].name} (this book)" selected={scope === here} onclick={() => setScope(here)} />
		{/if}
		<MenuSeparator />
		<div class="books">
			{#each books.filter((b) => testament === null || b.newTestament === testament) as b (b.index)}
				<MenuItem label={b.name} selected={scope === b.index} onclick={() => setScope(b.index)} />
			{/each}
		</div>
	</Menu>
{/if}

{#if orderMenu && orderButton}
	<Menu anchor={orderButton} quiet align="end" onclose={() => (orderMenu = false)} label="Order">
		<MenuItem
			label="Canonical"
			selected={!search.currentFirst}
			onclick={() => {
				search.currentFirst = false;
				orderMenu = false;
			}}
		/>
		<MenuItem
			label="This book first"
			selected={search.currentFirst}
			onclick={() => {
				search.currentFirst = true;
				orderMenu = false;
			}}
		/>
	</Menu>
{/if}

<style>
	.panel {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	/* 4 pt below, then the 6 pt the Mac leaves between it and the list. */
	.controls {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 8px 8px 10px;
	}

	.filters {
		display: flex;
		align-items: center;
		gap: 16px;
		height: 32px;
	}

	/* A pop-up button: the field colour, a hairline and a soft drop, the book glyph, the value and
	   up-down chevrons. */
	.select {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 168px;
		height: 32px;
		padding: 0 8px 0 12px;
		border-radius: 8px;
		background: var(--field);
		box-shadow:
			0 0 0 0.5px var(--border),
			0 1px 2px rgb(0 0 0 / 0.06);
		color: var(--ink);
		font: var(--type-ui);
		text-align: left;
	}

	.select:hover,
	.select.open {
		background: var(--raised);
	}

	.select :global(.lead),
	.select :global(.chevrons) {
		flex-shrink: 0;
		color: var(--ink-2);
	}

	.value {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.count {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 20px;
		margin: 0;
		padding: 4px 4px 0;
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.order {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.order :global(svg) {
		color: var(--ink-3);
	}

	.none {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 0 8px;
	}

	.suggestion {
		display: flex;
		flex-direction: column;
		align-items: center;
		min-width: 112px;
		height: 60px;
		margin-top: 20px;
		padding: 12px 16px 0;
		border-radius: 12px;
		background: var(--fill);
	}

	.suggestion:hover {
		background: var(--fill-strong);
	}

	.did {
		height: 16px;
		color: var(--ink-3);
		font: 500 12px/16px var(--font-ui);
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 16px;
		margin-top: 4px;
	}

	.q {
		color: var(--accent);
		font: 600 13px/16px var(--font-ui);
	}

	.detail {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.results {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}

	.track {
		position: relative;
	}

	.entry {
		position: absolute;
		left: 8px;
		right: 8px;
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 20px;
		margin-top: 8px;
		padding: 0 8px;
	}

	.book {
		color: var(--ink-3);
		font: var(--type-caption);
		letter-spacing: var(--type-caption-tracking);
	}

	.n {
		color: var(--ink-3);
		font: var(--type-small);
	}

	mark {
		padding: 0 2px;
		border-radius: 3px;
		background: var(--hl-yellow);
		color: var(--ink);
		font-weight: 500;
		-webkit-box-decoration-break: clone;
		box-decoration-break: clone;
	}

	.books {
		max-height: 280px;
		overflow-y: auto;
	}
</style>
