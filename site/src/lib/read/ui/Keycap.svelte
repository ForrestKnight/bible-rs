<script lang="ts">
	let { label, class: cls }: { label: string; class?: string } = $props();
</script>

<!-- A keycap: 20 pt high, radius 5, the fill with a hairline and a heavier bottom edge. -->
<kbd class={['keycap', cls]}>{label}</kbd>

<style>
	.keycap {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		min-width: 20px;
		height: 20px;
		padding: 0 5px;
		border-radius: 5px;
		background: var(--fill);
		box-shadow:
			0 0 0 0.5px var(--border),
			inset 0 -0.5px 0 var(--border);
		color: var(--ink-2);
		font: 500 11px/1 var(--font-ui);
		white-space: nowrap;
	}
</style>
