<script lang="ts">
	let { label }: { label: string } = $props();
</script>

<div class="header" role="presentation">{label}</div>

<style>
	.header {
		display: flex;
		align-items: flex-end;
		height: var(--menu-header-height);
		padding: 0 var(--menu-item-padding) 2px;
		color: var(--ink-3);
		font: var(--type-caption);
		white-space: nowrap;
	}
</style>
