<script lang="ts">
	import Icon from './Icon.svelte';
	import type { IconName } from './icons';

	interface Props {
		icon: IconName;
		label: string;
		/** 28 by default; below 26 the glyph is 12. */
		size?: number;
		on?: boolean;
		disabled?: boolean;
		onclick?: (e: MouseEvent) => void;
		el?: HTMLButtonElement;
		class?: string;
	}

	let { icon, label, size = 28, on = false, disabled = false, onclick, el = $bindable(), class: cls }: Props =
		$props();
</script>

<!-- A round icon button: no fill until hovered or pressed; a 16 pt glyph in secondary ink. -->
<button
	bind:this={el}
	type="button"
	class={['round', { on }, cls]}
	style:width="{size}px"
	style:height="{size}px"
	title={label}
	aria-label={label}
	{disabled}
	{onclick}
>
	<Icon name={icon} size={size < 26 ? 12 : 16} heavy />
</button>

<style>
	.round {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		border-radius: 50%;
		color: var(--ink-2);
		transition: background-color var(--motion-fast);
	}

	.round:hover:not(:disabled) {
		background: var(--fill);
	}

	.round:active:not(:disabled) {
		background: var(--fill-strong);
	}

	.round.on {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.round:disabled {
		color: var(--ink-3);
	}
</style>
