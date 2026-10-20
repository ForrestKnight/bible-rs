<script lang="ts">
	import { COMPARE } from '#lib/scripture.js';
	import Icon from './Icon.svelte';

	/** One verse in every installed translation, the current one first. `reveal` fades in each row. */
	let { reveal, rows = 5, footer = false }: { reveal: number[]; rows?: number; footer?: boolean } = $props();
</script>

<div class="compare">
	<div class="head">
		<div>
			<b>John 13:34</b>
			<span>{COMPARE.length} translations · current first</span>
		</div>
		<Icon name="copy" size={13} />
		<Icon name="close" size={13} />
	</div>
	<div class="rows">
		{#each COMPARE.slice(0, rows) as row, i (row.abbrev)}
			<div
				class={['row', { current: i === 0 }]}
				style:opacity={reveal[i]}
				style:transform="translateY({(1 - reveal[i]) * 6}px)"
			>
				<span class="abbrev">{row.abbrev}</span>
				<p>{row.text}</p>
			</div>
		{/each}
	</div>
	{#if footer}
		<div class="foot">
			<span>+ {COMPARE.slice(rows).map((r) => r.abbrev).join(', ')}</span>
			<span class="open"><Icon name="parallel" size={12} />Open in Parallel</span>
		</div>
	{/if}
</div>

<style>
	.compare {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: 12px;
		background: var(--popover);
		box-shadow: var(--popover-edge);
		color: var(--ink);
		font: 11px/14px var(--sans);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 11px 14px 10px;
		border-bottom: 1px solid var(--separator);
		color: var(--ink-2);
	}

	.head div {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 2px;
		font-size: 10px;
	}

	.head b {
		color: var(--ink);
		font-size: 13px;
		line-height: 16px;
	}

	.rows {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 6px;
	}

	.row {
		display: flex;
		gap: 12px;
		padding: 8px 10px;
		border-radius: 8px;
	}

	.row.current {
		background: var(--accent-soft);
	}

	.abbrev {
		flex-shrink: 0;
		width: 28px;
		padding-top: 2px;
		color: var(--ink-2);
		font-size: 9.5px;
		font-weight: 700;
	}

	.current .abbrev {
		color: var(--accent);
	}

	p {
		margin: 0;
		font: 11.5px/17px var(--serif);
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 7px 8px 7px 14px;
		border-top: 1px solid var(--separator);
		color: var(--ink-3);
		font-size: 10px;
	}

	.open {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 4px 9px;
		border-radius: 999px;
		background: var(--control);
		box-shadow: var(--control-edge);
		color: var(--ink);
		font-size: 10.5px;
		font-weight: 500;
	}
</style>
