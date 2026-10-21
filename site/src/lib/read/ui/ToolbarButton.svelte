<script lang="ts">
	import Icon from './Icon.svelte';
	import type { IconName } from './icons';

	interface Props {
		icon: IconName;
		/** The tooltip, which also names the button. */
		label: string;
		/** On: accent on accent-soft (a view that is showing). */
		on?: boolean;
		/** A toggle reports `on` as pressed. */
		toggle?: boolean;
		disabled?: boolean;
		/** Held down while its menu is open. */
		open?: boolean;
		width?: number;
		onclick?: (e: MouseEvent) => void;
		el?: HTMLButtonElement;
	}

	let {
		icon,
		label,
		on = false,
		toggle = false,
		disabled = false,
		open = false,
		width = 36,
		onclick,
		el = $bindable()
	}: Props = $props();
</script>

<button
	bind:this={el}
	type="button"
	class={['button', { on, open }]}
	style:width="{width}px"
	title={label}
	aria-label={label}
	aria-pressed={toggle ? on : undefined}
	aria-expanded={open || undefined}
	{disabled}
	{onclick}
>
	<Icon name={icon} size={20} />
</button>

<style>
	.button {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		height: var(--titlebar-capsule-height);
		border-radius: var(--titlebar-capsule-radius);
		color: var(--ink);
		transition: background-color var(--motion-fast);
	}

	.button:hover:not(:disabled),
	.button.open {
		background: var(--fill);
	}

	.button:active:not(:disabled) {
		background: var(--fill-strong);
	}

	.button.on,
	.button.on:hover {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.button:disabled {
		color: var(--ink-3);
	}
</style>
