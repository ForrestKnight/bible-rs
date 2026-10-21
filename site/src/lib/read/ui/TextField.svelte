<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import Icon from './Icon.svelte';
	import type { IconName } from './icons';

	interface Props extends Omit<HTMLInputAttributes, 'value' | 'class'> {
		value: string;
		icon?: IconName;
		/** Monospaced text (keys). */
		mono?: boolean;
		error?: boolean;
		class?: string;
		input?: HTMLInputElement;
	}

	let {
		value = $bindable(),
		icon,
		mono = false,
		error = false,
		class: cls,
		input = $bindable(),
		type = 'text',
		...rest
	}: Props = $props();
</script>

<!-- A text field: 32 pt, radius 8, a hairline; focused, an accent outline and the focus ring;
     in error, a danger outline and halo. -->
<div class={['field', { error, mono }, cls]}>
	{#if icon}<Icon name={icon} size={16} heavy class="lead" />{/if}
	<input bind:this={input} bind:value {type} spellcheck="false" aria-invalid={error || undefined} {...rest} />
</div>

<style>
	.field {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 0 10px;
		border-radius: 8px;
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--ink);
		transition: box-shadow var(--motion-fast);
	}

	.field:hover {
		box-shadow: inset 0 0 0 1px var(--ink-3);
	}

	.field:focus-within {
		box-shadow:
			inset 0 0 0 1px var(--accent),
			0 0 0 3px var(--focus);
	}

	.field.error {
		box-shadow:
			inset 0 0 0 1px var(--danger),
			0 0 0 3px color-mix(in srgb, var(--danger) 25%, transparent);
	}

	.field :global(.lead) {
		color: var(--ink-3);
	}

	input {
		flex: 1;
		min-width: 0;
		height: 100%;
		padding: 0;
		border: 0;
		outline: none;
		background: none;
		font: var(--type-ui);
	}

	.mono input:not(:placeholder-shown) {
		font: 13px/20px ui-monospace, SFMono-Regular, Menlo, monospace;
	}

	input::placeholder {
		color: var(--ink-3);
		opacity: 1;
	}
</style>
