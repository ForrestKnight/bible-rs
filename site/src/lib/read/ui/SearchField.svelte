<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import Icon from './Icon.svelte';

	interface Props extends Omit<HTMLInputAttributes, 'value' | 'class' | 'style'> {
		value: string;
		placeholder?: string;
		/** Glass in the toolbar; the field colour inside panels and pages. */
		glass?: boolean;
		width?: number | string;
		/** Called when the clear button empties the field. */
		oncleared?: () => void;
		input?: HTMLInputElement;
		class?: string;
	}

	let {
		value = $bindable(),
		placeholder = 'Search',
		glass = true,
		width,
		oncleared,
		input = $bindable(),
		class: cls,
		...rest
	}: Props = $props();

	function clear() {
		value = '';
		input?.focus();
		oncleared?.();
	}
</script>

<!-- The toolbar search field: a capsule with a magnifier; focused, the field colour, an accent
     outline and the focus ring; with text, a clear button. -->
<div
	class={['field', { glass }, cls]}
	style:width={typeof width === 'number' ? `${width}px` : width}
>
	<Icon name="search" size={16} class="glass-icon" />
	<input bind:this={input} bind:value type="search" {placeholder} autocomplete="off" spellcheck="false" {...rest} />
	{#if value}
		<button type="button" class="clear" aria-label="Clear" tabindex="-1" onclick={clear}>
			<Icon name="clear" size={16} />
		</button>
	{/if}
</div>

<style>
	.field {
		position: relative;
		display: flex;
		align-items: center;
		flex-shrink: 0;
		height: var(--titlebar-capsule-height);
		padding: 0 10px;
		border-radius: var(--titlebar-capsule-radius);
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--ink);
		transition: box-shadow var(--motion-fast);
	}

	.field.glass {
		background: var(--glass);
		box-shadow: var(--shadow-control);
		-webkit-backdrop-filter: blur(20px) saturate(1.4);
		backdrop-filter: blur(20px) saturate(1.4);
	}

	.field:focus-within {
		background: var(--field);
		box-shadow:
			inset 0 0 0 1px var(--accent),
			0 0 0 3px var(--focus);
	}

	.field :global(.glass-icon) {
		color: var(--ink-2);
	}

	input {
		flex: 1;
		min-width: 0;
		height: 100%;
		margin-left: 6px;
		padding: 0;
		border: 0;
		outline: none;
		background: none;
		font: var(--type-ui);
	}

	input::placeholder {
		color: var(--ink-3);
		opacity: 1;
	}

	input::-webkit-search-cancel-button,
	input::-webkit-search-decoration {
		appearance: none;
	}

	.clear {
		display: grid;
		place-items: center;
		margin-left: 4px;
		margin-right: 0;
		color: var(--ink-3);
		--icon-cut: var(--paper);
	}

	.clear:hover {
		color: var(--ink-2);
	}
</style>
