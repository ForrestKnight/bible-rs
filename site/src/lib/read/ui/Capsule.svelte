<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** Hairlines between the buttons (`|` between each child). */
		dividers?: boolean;
		class?: string;
		style?: string;
		label?: string;
		children: Snippet;
	}

	let { dividers = false, class: cls, style, label, children }: Props = $props();
</script>

<!-- The glass capsule behind toolbar controls: 32 tall, the glass colour over the control shadow. -->
<div class={['capsule', { dividers }, cls]} {style} role={label ? 'group' : undefined} aria-label={label}>
	{@render children()}
</div>

<style>
	.capsule {
		display: flex;
		align-items: center;
		height: var(--titlebar-capsule-height);
		border-radius: var(--titlebar-capsule-radius);
		background: var(--glass);
		box-shadow: var(--shadow-control);
		-webkit-backdrop-filter: blur(20px) saturate(1.4);
		backdrop-filter: blur(20px) saturate(1.4);
	}

	.dividers > :global(* + *) {
		position: relative;
		margin-left: 1px;
	}

	.dividers > :global(* + *)::before {
		content: '';
		position: absolute;
		left: -1px;
		top: 8px;
		width: 1px;
		height: 16px;
		background: var(--separator);
	}
</style>
