<script lang="ts">
	import { RESULTS } from '#lib/demo/script.js';

	/**
	 * The search sidebar: scope, the result count and the matching verses with the hits marked.
	 * `reveal` fades in each result (0–1); `found` shows the count line.
	 */
	let {
		reveal,
		found,
		selected,
		top = 10
	}: { reveal: number[]; found: number; selected: number; top?: number } = $props();

	const results = RESULTS.map((r) => ({
		...r,
		parts: r.hits.split(/\[|\]/).map((text, i) => ({ text, hit: i % 2 === 1 }))
	}));
</script>

<div class="panel" style:padding-top="{top}px">
	<div class="segmented">
		<span class="on">Bible</span><span>Old Testament</span><span>New Testament</span>
	</div>
	<div class="scope">
		<span class="select">All books</span>
		<span class="check"><span class="box"></span>Whole words</span>
	</div>
	<div class="count" style:opacity={found}>
		<span>19 verses in 9 books</span><span>Canonical</span>
	</div>
	<div class="group" style:opacity={found}><span>John</span><span>4</span></div>
	{#each results as r, i (r.ref)}
		{#if i === 4}
			<div class="group" style:opacity={reveal[i]}><span>Romans</span><span>2</span></div>
		{/if}
		<div
			class={['result', { selected: i === selected }]}
			style:opacity={reveal[i]}
			style:transform="translateY({(1 - reveal[i]) * 6}px)"
		>
			<div class="ref"><span>{r.ref}</span><span class="tr">KJV</span></div>
			<p>
				{#each r.parts as part, j (j)}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}
			</p>
		</div>
	{/each}
</div>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: 2px;
		height: 100%;
		padding: 10px 8px;
		overflow: hidden;
		border-radius: 12px;
		background: var(--sidebar);
		box-shadow: 0 0 0 0.5px var(--separator);
		color: var(--ink);
		font: 11px/14px var(--sans);
	}

	.segmented {
		display: flex;
		gap: 2px;
		margin-bottom: 8px;
		padding: 2px;
		border-radius: 7px;
		background: color-mix(in srgb, var(--ink) 6%, transparent);
		color: var(--ink-2);
		font-size: 10px;
		font-weight: 500;
	}

	.segmented span {
		flex: auto;
		padding: 4px 6px;
		border-radius: 5px;
		text-align: center;
		white-space: nowrap;
	}

	.segmented .on {
		background: var(--control);
		box-shadow: var(--control-edge);
		color: var(--ink);
		font-weight: 600;
	}

	.scope {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-bottom: 10px;
		color: var(--ink);
		font-size: 10.5px;
	}

	.select {
		flex: 0 0 52%;
		padding: 5px 8px;
		border-radius: 6px;
		background: var(--control);
		box-shadow: var(--control-edge);
	}

	.check {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.box {
		width: 11px;
		height: 11px;
		border-radius: 3px;
		box-shadow: inset 0 0 0 1px var(--ink-3);
	}

	.count,
	.group,
	.ref {
		display: flex;
		justify-content: space-between;
	}

	.count {
		padding: 0 4px 8px;
		color: var(--ink-2);
		font-size: 10px;
	}

	.group {
		padding: 6px 4px 2px;
		color: var(--ink-3);
		font-size: 9.5px;
		font-weight: 600;
	}

	.result {
		padding: 7px 10px 8px;
		border-radius: 8px;
	}

	.result.selected {
		background: var(--accent-soft);
	}

	.ref {
		margin-bottom: 3px;
		font-weight: 600;
	}

	.selected .ref span:first-child {
		color: var(--accent);
	}

	.tr {
		color: var(--ink-3);
		font-size: 9.5px;
		font-weight: 400;
	}

	p {
		display: -webkit-box;
		margin: 0;
		overflow: hidden;
		color: var(--ink-2);
		font: 10.5px/17px var(--serif);
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
	}

	mark {
		border-radius: 2px;
		background: var(--hl-yellow);
		color: var(--ink);
	}
</style>
