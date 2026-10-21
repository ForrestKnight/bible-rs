<script lang="ts">
	import { onMount, tick, type Snippet } from 'svelte';
	import { focusables, portal, trapTab } from './float';

	interface Props {
		label: string;
		onclose: () => void;
		width?: number;
		height?: number;
		/** A click on the scrim closes it (sheets with unsaved input don't). */
		dismissable?: boolean;
		radius?: number;
		/** Hangs the sheet this far from the top instead of centring it. */
		top?: number;
		/** Focuses the sheet itself on opening rather than its first control. */
		focusSheet?: boolean;
		class?: string;
		children: Snippet;
	}

	let {
		label,
		onclose,
		width = 560,
		height,
		dismissable = true,
		radius = 16,
		top,
		focusSheet = false,
		class: cls,
		children
	}: Props = $props();

	let panel = $state<HTMLElement>();

	onMount(() => {
		const before = document.activeElement as HTMLElement | null;
		void tick().then(() => {
			if (!panel) return;
			const target = focusSheet
				? panel
				: (panel.querySelector<HTMLElement>('[data-autofocus]') ?? focusables(panel)[0] ?? panel);
			target.focus({ preventScroll: true });
		});
		return () => {
			if (before?.isConnected) before.focus({ preventScroll: true });
		};
	});

	function keydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			onclose();
		} else if (panel) {
			trapTab(panel, e);
		}
	}
</script>

<div use:portal class="layer">
	<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
	<div class="scrim" onclick={() => dismissable && onclose()}></div>
	<div
		bind:this={panel}
		class={['sheet', cls]}
		role="dialog"
		aria-modal="true"
		aria-label={label}
		tabindex="-1"
		style:width="min({width}px, calc(100vw - 32px))"
		style:height={height ? `min(${height}px, calc(100dvh - 32px))` : undefined}
		style:border-radius="{radius}px"
		style:align-self={top === undefined ? undefined : 'start'}
		style:margin-top={top === undefined ? undefined : `${top}px`}
		onkeydown={keydown}
	>
		{@render children()}
	</div>
</div>

<style>
	.layer {
		position: fixed;
		inset: 0;
		z-index: 80;
		display: grid;
		place-items: center;
	}

	.scrim {
		position: absolute;
		inset: 0;
		background: var(--scrim);
		animation: fade var(--motion-base) var(--motion-easing);
	}

	.sheet {
		position: relative;
		display: flex;
		flex-direction: column;
		max-height: calc(100dvh - 32px);
		overflow: hidden;
		background: var(--surface);
		box-shadow: var(--shadow-modal);
		color: var(--ink);
		font: var(--type-ui);
		outline: none;
		animation: rise var(--motion-base) var(--motion-easing);
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(8px) scale(0.98);
		}
	}
</style>
