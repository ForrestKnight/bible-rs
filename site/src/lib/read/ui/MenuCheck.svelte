<script lang="ts">
	interface Props {
		label: string;
		checked: boolean;
		onchange: (checked: boolean) => void;
	}

	let { label, checked, onchange }: Props = $props();
</script>

<button
	type="button"
	class="item"
	role="menuitemcheckbox"
	aria-checked={checked}
	tabindex="-1"
	onclick={() => onchange(!checked)}
>
	<span class={['box', { on: checked }]}>
		{#if checked}
			<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M4 8.25 7 11 12 5.25" /></svg>
		{/if}
	</span>
	<span class="label">{label}</span>
</button>

<style>
	.item {
		display: flex;
		align-items: center;
		gap: 8px;
		height: var(--menu-item-height);
		padding: 0 var(--menu-item-padding);
		border-radius: var(--menu-item-radius);
		color: var(--ink);
		text-align: left;
		white-space: nowrap;
		outline: none;
	}

	.item:focus {
		background: var(--fill);
	}

	.box {
		display: block;
		flex-shrink: 0;
		width: 16px;
		height: 16px;
		border-radius: 4px;
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
	}

	.box.on {
		background: var(--accent);
		box-shadow: none;
	}

	svg {
		display: block;
		fill: none;
		stroke: var(--on-accent);
		stroke-width: 1.75;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
