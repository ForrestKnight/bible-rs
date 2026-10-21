<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import Icon from './Icon.svelte';
	import type { IconName } from './icons';

	interface Props extends Omit<HTMLButtonAttributes, 'class'> {
		kind?: 'primary' | 'secondary' | 'plain' | 'ghost' | 'destructive';
		/** sm 24 (12 pt, 10 pt sides), md 32 (13 pt, 14), lg 40 (13 pt, 20). */
		size?: 'sm' | 'md' | 'lg';
		icon?: IconName;
		/** The glyph after the label. */
		trailing?: boolean;
		label?: string;
		class?: string;
		el?: HTMLButtonElement;
		children?: Snippet;
	}

	let {
		kind = 'secondary',
		size = 'md',
		icon,
		trailing = false,
		label,
		class: cls,
		el = $bindable(),
		children,
		type = 'button',
		...rest
	}: Props = $props();
</script>

<button bind:this={el} {type} class={['button', kind, size, cls]} {...rest}>
	{#if icon && !trailing}<Icon name={icon} size={16} heavy />{/if}
	{#if children}{@render children()}{:else if label}<span>{label}</span>{/if}
	{#if icon && trailing}<Icon name={icon} size={16} heavy />{/if}
</button>

<style>
	.button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		flex-shrink: 0;
		height: 32px;
		padding: 0 14px;
		border-radius: 16px;
		font: 500 13px/20px var(--font-ui);
		white-space: nowrap;
		transition:
			background-color var(--motion-fast),
			box-shadow var(--motion-fast);
	}

	.sm {
		height: 24px;
		padding: 0 10px;
		border-radius: 12px;
		font-size: 12px;
		line-height: 16px;
	}

	.lg {
		height: 40px;
		padding: 0 20px;
		border-radius: 20px;
	}

	.primary {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.secondary {
		background: var(--raised);
		box-shadow: var(--shadow-control);
		color: var(--ink);
	}

	.plain {
		background: var(--fill);
		color: var(--ink);
	}

	.ghost {
		color: var(--accent);
	}

	.destructive {
		background: var(--danger);
		color: #fff;
		font-weight: 600;
	}

	.secondary:hover:not(:disabled),
	.ghost:hover:not(:disabled) {
		background-image: linear-gradient(var(--fill), var(--fill));
	}

	.plain:hover:not(:disabled) {
		background: var(--fill-strong);
	}

	.secondary:active:not(:disabled),
	.ghost:active:not(:disabled) {
		background-image: linear-gradient(var(--fill-strong), var(--fill-strong));
	}

	.primary:hover:not(:disabled),
	.destructive:hover:not(:disabled) {
		background-image: linear-gradient(var(--hover-wash), var(--hover-wash));
	}

	.primary:active:not(:disabled),
	.destructive:active:not(:disabled) {
		background-image: linear-gradient(var(--pressed-wash), var(--pressed-wash));
	}

	.button:disabled {
		opacity: var(--disabled-opacity);
	}
</style>
