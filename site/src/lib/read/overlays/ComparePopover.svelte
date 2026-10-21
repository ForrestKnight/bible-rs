<!--
	Compare as a popover (compare_popover.rs): the selected verse in every translation, the current
	one first, hanging from the verse in the reader. The rows that don't fit are named in the
	footer. (With parallel columns open, Compare uses the verse panel instead.)
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Button, Popover, RoundButton } from '../ui';

	const WIDTH = 400;
	const BEAK = 9;
	/** The header (60), the list's padding (12) and the footer (46). */
	const CHROME = 60 + 12 + 46;
	const TOOLBAR = 52;

	const app = getApp();
	const verse = app.compare!;

	let heights = $state<number[]>([]);
	let geom = $state({ below: 0, above: 0, tip: 60 });

	const reference = $derived(app.label(verse));
	const rows = $derived.by(() => {
		const core = app.core;
		const current = app.settings.translation;
		if (!core) return [];
		const ids = [current, ...app.translations.map((t) => t.id).filter((id) => id !== current)];
		return ids.flatMap((id) => {
			if (!app.loaded.has(id)) return [];
			const v = core.verse(id, verse);
			if (!v) return [];
			return [{ id, abbrev: app.info(id)?.abbrev ?? id.toUpperCase(), text: v.runs.map((r) => r.text).join('') }];
		});
	});
	/** As many rows as fit under the verse (or above it), at least one. */
	const shown = $derived.by(() => {
		const room = Math.max(geom.below, geom.above) - CHROME;
		let n = 0;
		let h = 0;
		for (const rh of heights.slice(0, rows.length)) {
			if (h + rh > room && n > 0) break;
			h += rh;
			n++;
		}
		return Math.max(n, Math.min(1, rows.length));
	});
	const listHeight = $derived(heights.slice(0, shown).reduce((s, h) => s + h, 0));
	const downward = $derived(geom.below >= CHROME + listHeight || geom.below >= geom.above);
	const rest = $derived(rows.slice(shown).map((r) => r.abbrev));

	/** The selected verse on screen, or where it would be. */
	function verseRect(): DOMRect {
		return app.reader?.selectionRect() ?? new DOMRect(innerWidth / 2 + 190, 190, 20, 20);
	}

	function anchor(): DOMRect {
		const a = verseRect();
		const left = Math.max(12, Math.min(a.left + a.width / 2 - WIDTH / 2, innerWidth - 12 - WIDTH));
		geom = {
			below: innerHeight - (a.bottom + 3 + BEAK) - 16,
			above: a.top - 3 - BEAK - TOOLBAR - 8,
			tip: Math.min(Math.max(left + 60, a.left + 24), left + WIDTH - 24) - left
		};
		return new DOMRect(left, a.top, 0, a.height);
	}

	const close = () => (app.compare = null);

	async function copyAll() {
		const text = `${reference}\n${rows.map((r) => `${r.abbrev}: ${r.text}`).join('\n')}`;
		try {
			await navigator.clipboard.writeText(text);
			app.toastNow(`Copied ${reference} in ${rows.length} translations`);
		} catch {
			app.toastNow("Couldn't copy");
		}
	}

	function openParallel() {
		for (const r of rows.slice(1, shown)) app.dispatch({ type: 'addParallel', id: r.id });
		if (!app.settings.parallelOn) app.dispatch({ type: 'toggleParallel' });
		close();
	}
</script>

<Popover
	{anchor}
	side={downward ? 'bottom' : 'top'}
	gap={3 + BEAK}
	flip={false}
	onclose={close}
	label="Compare {reference}"
	radius={16}
	style="width: {WIDTH}px"
>
	<svg
		class={['beak', { up: !downward }]}
		style:left="{geom.tip - 9}px"
		width="18"
		height="10"
		viewBox="0 0 18 10"
		aria-hidden="true"
	>
		<path d="M0 9.5 9 0.5 18 9.5" />
	</svg>
	<header class="head">
		<div class="titles">
			<h2 class="title">{reference}</h2>
			<p class="sub">{rows.length} {rows.length === 1 ? 'translation' : 'translations'} · current first</p>
		</div>
		<RoundButton icon="copy" label="Copy all" onclick={copyAll} />
		<RoundButton icon="close" label="Close" onclick={close} />
	</header>
	<div class="list">
		{#each rows as r, i (r.id)}
			{@const current = r.id === app.settings.translation}
			<button
				type="button"
				class={['row', { current, spill: i >= shown }]}
				bind:offsetHeight={heights[i]}
				aria-hidden={i >= shown || undefined}
				tabindex={i >= shown ? -1 : undefined}
				aria-current={current || undefined}
				onclick={() => !current && app.dispatch({ type: 'setTranslation', id: r.id })}
			>
				<span class="abbrev">{r.abbrev}</span>
				<span class="text">{r.text}</span>
			</button>
		{/each}
	</div>
	<footer class="foot">
		{#if rest.length}<p class="rest">+ {rest.join(', ')}</p>{/if}
		<Button size="sm" icon="columns" label="Open in Parallel" onclick={openParallel} />
	</footer>
</Popover>

<style>
	.beak {
		position: absolute;
		top: -9px;
		overflow: visible;
	}

	.beak.up {
		top: auto;
		bottom: -9px;
		transform: scaleY(-1);
	}

	.beak path {
		fill: var(--surface);
		stroke: var(--border);
		stroke-width: 0.75;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 60px;
		padding: 0 12px 0 16px;
		box-shadow: inset 0 -1px 0 var(--separator);
	}

	.titles {
		flex: 1;
		min-width: 0;
		align-self: flex-start;
		padding-top: 14px;
	}

	.title {
		margin: 0;
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.sub {
		margin: 0;
		color: var(--ink-2);
		font: 400 12px/16px var(--font-ui);
	}

	.list {
		position: relative;
		display: flex;
		flex-direction: column;
		padding: 6px 8px;
	}

	.row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		padding: 8px;
		border-radius: 8px;
		text-align: left;
	}

	.row:hover:not(.current) {
		background: var(--fill);
	}

	.row.current {
		background: var(--accent-soft);
		cursor: default;
	}

	.row.spill {
		position: absolute;
		left: 8px;
		right: 8px;
		visibility: hidden;
	}

	.abbrev {
		flex-shrink: 0;
		width: 36px;
		color: var(--ink-2);
		font: 700 11px/20px var(--font-ui);
	}

	.current .abbrev {
		color: var(--accent);
	}

	.text {
		display: -webkit-box;
		flex: 1;
		min-width: 0;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 8;
		line-clamp: 8;
		color: var(--ink);
		font: 400 14px/20px var(--font-reading);
	}

	.foot {
		display: flex;
		align-items: center;
		height: 46px;
		padding: 0 12px 0 16px;
		box-shadow: inset 0 1px 0 var(--separator);
	}

	.rest {
		margin: 0;
		color: var(--ink-3);
		font: 400 12px/16px var(--font-ui);
	}

	.foot :global(.button) {
		margin-left: auto;
	}
</style>
