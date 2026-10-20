<script lang="ts">
	/** Keycaps as the app draws them in menus and the palette. */
	let { keys, size = 18 }: { keys: string[]; size?: number } = $props();
</script>

<span class="keys">
	{#each keys as key, i (i)}
		<kbd style:min-width="{size}px" style:height="{size}px">{key}</kbd>
	{/each}
</span>

<style>
	.keys {
		display: inline-flex;
		flex-shrink: 0;
		gap: 3px;
	}

	kbd {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0 4px;
		border-radius: 4px;
		background: var(--chip);
		box-shadow: var(--chip-edge);
		color: var(--ink-2);
		font: 500 9.5px/1 var(--sans);
	}
</style>
