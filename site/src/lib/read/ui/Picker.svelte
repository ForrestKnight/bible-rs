<script lang="ts">
	import Icon from './Icon.svelte';

	interface Props {
		text: string;
		/** The tooltip and accessible description. */
		label: string;
		padLeft?: number;
		padRight?: number;
		open?: boolean;
		onclick?: (e: MouseEvent) => void;
		el?: HTMLButtonElement;
	}

	let { text, label, padLeft = 14, padRight = 12, open = false, onclick, el = $bindable() }: Props = $props();
</script>

<!-- A picker inside a capsule: 13 pt semibold, then a 12 pt chevron 4 pt after. -->
<button
	bind:this={el}
	type="button"
	class={['picker', { open }]}
	style:padding="0 {padRight}px 0 {padLeft}px"
	title={label}
	aria-haspopup="true"
	aria-expanded={open}
	{onclick}
>
	<span class="text">{text}</span>
	<Icon name="chevron-down" size={12} class="chevron" />
</button>

<style>
	.picker {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
		height: var(--titlebar-capsule-height);
		border-radius: var(--titlebar-capsule-radius);
		color: var(--ink);
		font: var(--type-ui-strong);
		white-space: nowrap;
		transition: background-color var(--motion-fast);
	}

	.picker:hover {
		background: var(--fill);
	}

	.picker:active,
	.picker.open {
		background: var(--fill-strong);
	}

	.picker :global(.chevron) {
		color: var(--ink-2);
	}
</style>
