<script lang="ts" module>
	/** The six highlight colours, as their swatches. */
	export const SWATCHES = ['#ebc24a', '#6dbe7e', '#5b9be6', '#e88aae', '#a58be0', '#ee9a55'];
</script>

<script lang="ts">
	import Icon, { type IconName } from './Icon.svelte';

	/** The verse menu: highlight colours, then the actions for the selected verses. */
	let { picked = -1, hover = -1 }: { picked?: number; hover?: number } = $props();

	const ITEMS: ({ icon: IconName; label: string; key: string; dim?: boolean } | null)[] = [
		{ icon: 'note', label: 'Add Note…', key: 'N' },
		{ icon: 'bookmark', label: 'Bookmark', key: 'B' },
		null,
		{ icon: 'copy', label: 'Copy', key: '⌘C' },
		{ icon: 'quote', label: 'Copy with Reference', key: '⇧⌘C' },
		{ icon: 'share', label: 'Share as Image…', key: '' },
		null,
		{ icon: 'compare', label: 'Compare Translations', key: 'C' },
		{ icon: 'parallel', label: 'Open in Parallel', key: '⇧⌘P' },
		null,
		{ icon: 'range', label: 'Select Verse Range', key: '⇧-click', dim: true }
	];
</script>

<div class="menu">
	<div class="label">Highlight</div>
	<div class="swatches">
		{#each SWATCHES as color, i (color)}
			<span class={['swatch', { on: i === picked }]} style:background={color}>
				{#if i === picked}<Icon name="check" size={9} />{/if}
			</span>
		{/each}
		<span class="none"><Icon name="none" size={16} /></span>
	</div>
	{#each ITEMS as item, i (i)}
		{#if item}
			<div class={['item', { hover: i === hover, dim: item.dim }]}>
				<Icon name={item.icon} size={12} />
				<span class="name">{item.label}</span>
				<span class="key">{item.key}</span>
			</div>
		{:else}
			<div class="sep"></div>
		{/if}
	{/each}
</div>

<style>
	.menu {
		display: flex;
		flex-direction: column;
		width: 206px;
		padding: 8px 5px 5px;
		border-radius: 10px;
		background: var(--popover);
		box-shadow: var(--popover-edge);
		color: var(--ink);
		font: 11px/14px var(--sans);
	}

	.label {
		padding: 0 9px;
		color: var(--ink-2);
		font-size: 10px;
		font-weight: 600;
	}

	.swatches {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 7px 9px 8px;
	}

	.swatch {
		display: grid;
		place-items: center;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		color: #1c1d21;
		transition: box-shadow 0.2s;
	}

	.swatch.on {
		box-shadow:
			0 0 0 1.5px var(--popover),
			0 0 0 3px var(--ink);
	}

	.swatch :global(svg) {
		stroke-width: 2.6;
	}

	.none {
		display: grid;
		color: var(--ink-3);
	}

	.item {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 23px;
		padding: 0 9px;
		border-radius: 5px;
	}

	.item.hover {
		background: var(--menu-pick);
		color: #fff;
	}

	.item.dim {
		color: var(--ink-3);
	}

	.name {
		flex: 1;
	}

	.key {
		color: var(--ink-3);
		font-size: 10.5px;
	}

	.hover .key {
		color: rgba(255, 255, 255, 0.8);
	}

	.sep {
		height: 1px;
		margin: 4px 9px;
		background: var(--separator);
	}
</style>
