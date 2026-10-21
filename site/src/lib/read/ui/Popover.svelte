<script lang="ts">
	import { onMount, tick, type Snippet } from 'svelte';
	import type { ClassValue } from 'svelte/elements';
	import { place, portal, type Align, type Side } from './float';

	interface Props {
		/** What it opens from: an element, a rectangle, or a function giving the rectangle now. */
		anchor: HTMLElement | DOMRect | (() => DOMRect | null) | null;
		side?: Side;
		align?: Align;
		gap?: number;
		/** Turns to the other side when there's no room (otherwise it's only pushed inside). */
		flip?: boolean;
		/** Escape, a click outside (not on the anchor) or Tab out of a menu. */
		onclose?: () => void;
		role?: 'dialog' | 'menu' | 'listbox';
		label?: string;
		/** Moves focus into the popover when it opens (its first field, or the panel). */
		autofocus?: boolean;
		/** The popover chrome: the surface, its ring and shadow, the radius. */
		frame?: boolean;
		radius?: number;
		class?: ClassValue;
		style?: string;
		children: Snippet;
		panel?: HTMLElement;
	}

	let {
		anchor,
		side = 'bottom',
		align = 'start',
		gap = 6,
		flip = true,
		onclose,
		role = 'dialog',
		label,
		autofocus = true,
		frame = true,
		radius = 12,
		class: cls,
		style,
		children,
		panel = $bindable()
	}: Props = $props();

	let pos = $state<{ x: number; y: number; side: Side } | null>(null);

	function anchorRect(): DOMRect | null {
		if (!anchor) return null;
		if (anchor instanceof HTMLElement) return anchor.getBoundingClientRect();
		if (typeof anchor === 'function') return anchor();
		return anchor;
	}

	function update() {
		const r = anchorRect();
		if (!r || !panel) return;
		pos = place(r, panel.offsetWidth, panel.offsetHeight, side, align, gap, flip);
	}

	onMount(() => {
		const before = document.activeElement as HTMLElement | null;
		update();
		const ro = new ResizeObserver(update);
		if (panel) ro.observe(panel);
		addEventListener('resize', update);
		addEventListener('scroll', update, true);
		if (autofocus) {
			void tick().then(() => {
				if (panel?.contains(document.activeElement)) return;
				const target =
					panel?.querySelector<HTMLElement>(
						'[data-autofocus], input:not([type]), input[type="text"], input[type="search"], textarea'
					) ?? panel;
				target?.focus({ preventScroll: true });
			});
		}
		return () => {
			ro.disconnect();
			removeEventListener('resize', update);
			removeEventListener('scroll', update, true);
			if (panel?.contains(document.activeElement) && before?.isConnected) before.focus({ preventScroll: true });
		};
	});

	$effect(() => {
		void anchor;
		update();
	});

	function outside(e: PointerEvent) {
		const t = e.target as Node;
		if (panel?.contains(t)) return;
		if (anchor instanceof HTMLElement && anchor.contains(t)) return;
		onclose?.();
	}

	function keydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			onclose?.();
		}
	}
</script>

<svelte:document onpointerdowncapture={outside} />

<div
	bind:this={panel}
	use:portal
	class={['popover', { frame }, cls]}
	data-side={pos?.side ?? side}
	{role}
	aria-label={label}
	tabindex="-1"
	style:left="{pos?.x ?? 0}px"
	style:top="{pos?.y ?? 0}px"
	style:visibility={pos ? 'visible' : 'hidden'}
	style:--radius="{radius}px"
	{style}
	onkeydown={keydown}
>
	{@render children()}
</div>

<style>
	.popover {
		position: fixed;
		z-index: 60;
		outline: none;
		color: var(--ink);
		font: var(--type-ui);
		animation: pop var(--motion-fast) var(--motion-easing);
	}

	.frame {
		background: var(--surface);
		border-radius: var(--radius);
		box-shadow: var(--shadow-popover);
	}

	@keyframes pop {
		from {
			opacity: 0;
			transform: translateY(-2px);
		}
	}

	.popover[data-side='top'] {
		animation-name: pop-up;
	}

	@keyframes pop-up {
		from {
			opacity: 0;
			transform: translateY(2px);
		}
	}
</style>
