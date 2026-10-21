<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';
	import type { IconName } from './icons';

	interface Props {
		icon?: IconName;
		label: string;
		/** Keycaps at the trailing edge, as text ("⇧⌘C"). */
		shortcut?: string;
		disabled?: boolean;
		/** The current choice (a radio-like row): a check before the label. */
		checked?: boolean;
		/** Leaves the check column empty when no row in the menu has an icon. */
		checkColumn?: boolean;
		/** The current choice in a quiet menu: the selection colour behind it, no check. */
		selected?: boolean;
		onclick: (e: MouseEvent) => void;
		/** Something drawn before the label instead of an icon (a swatch). */
		lead?: Snippet;
	}

	let {
		icon,
		label,
		shortcut,
		disabled = false,
		checked,
		checkColumn = false,
		selected,
		onclick,
		lead
	}: Props = $props();
</script>

<button
	type="button"
	class={['item', { selected }]}
	role={checked === undefined && selected === undefined ? 'menuitem' : 'menuitemradio'}
	aria-checked={checked ?? selected}
	aria-disabled={disabled || undefined}
	tabindex="-1"
	onclick={(e) => !disabled && onclick(e)}
>
	{#if lead}
		{@render lead()}
	{:else if icon}
		<Icon name={icon} size={16} heavy class="glyph" />
	{:else if checked !== undefined || checkColumn}
		<span class="check">{#if checked}<Icon name="check" size={16} />{/if}</span>
	{/if}
	<span class="label">{label}</span>
	{#if shortcut}<span class="shortcut">{shortcut}</span>{/if}
</button>

<style>
	.item {
		display: flex;
		align-items: center;
		gap: var(--menu-icon-gap);
		height: var(--menu-item-height);
		padding: 0 var(--menu-item-padding);
		border-radius: var(--menu-item-radius);
		color: var(--ink);
		text-align: left;
		white-space: nowrap;
		outline: none;
	}

	.item :global(.glyph),
	.check {
		color: var(--ink-2);
	}

	.check {
		display: block;
		width: 16px;
		height: 16px;
		color: var(--ink);
	}

	.label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: pre;
	}

	.shortcut {
		margin-left: 24px;
		color: var(--ink-3);
	}

	.item:focus {
		background: var(--accent);
		color: var(--on-accent);
	}

	.item:focus :global(.glyph),
	.item:focus .shortcut,
	.item:focus .check {
		color: var(--on-accent);
	}

	.item:focus:active {
		background: linear-gradient(rgb(0 0 0 / 0.07), rgb(0 0 0 / 0.07)), var(--accent);
	}

	:global(.quiet) .item:focus {
		background: var(--fill);
		color: var(--ink);
	}

	:global(.quiet) .item:focus :global(.glyph) {
		color: var(--ink-2);
	}

	:global(.quiet) .item:focus .shortcut {
		color: var(--ink-3);
	}

	:global(.quiet) .item:focus:active {
		background: var(--fill-strong);
	}

	.item.selected,
	:global(.quiet) .item.selected:focus {
		background: var(--selection);
	}

	.item[aria-disabled='true'],
	.item[aria-disabled='true'] :global(.glyph),
	.item[aria-disabled='true'] .shortcut {
		color: var(--ink-3);
	}
</style>
