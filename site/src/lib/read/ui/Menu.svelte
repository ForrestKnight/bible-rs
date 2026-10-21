<script lang="ts">
	import { onMount, tick, type Snippet } from 'svelte';
	import Popover from './Popover.svelte';
	import type { Align, Side } from './float';

	interface Props {
		anchor: HTMLElement | DOMRect | (() => DOMRect | null) | null;
		side?: Side;
		align?: Align;
		gap?: number;
		flip?: boolean;
		onclose: () => void;
		label?: string;
		/** At least this wide (rows stretch to the widest). */
		width?: number;
		/** Rows lit with a fill rather than the accent, the current one in the selection colour
		 *  (the Mac's egui menus: translations, column heads, the book picker). */
		quiet?: boolean;
		/** The row lit when it opens (as from the keyboard). */
		highlight?: number;
		class?: string;
		children: Snippet;
	}

	let {
		anchor,
		side = 'bottom',
		align = 'start',
		gap = 6,
		flip = true,
		onclose,
		label,
		width,
		quiet = false,
		highlight,
		class: cls,
		children
	}: Props = $props();

	let panel = $state<HTMLElement>();

	const items = () =>
		[...(panel?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])') ?? [])];

	onMount(() => {
		if (highlight === undefined) return;
		void tick().then(() => items()[highlight]?.focus({ preventScroll: true }));
	});

	/** Arrow keys move the highlight (focus), as the pointer does; Tab closes, as on the Mac. */
	function keydown(e: KeyboardEvent) {
		const list = items();
		const at = list.indexOf(document.activeElement as HTMLElement);
		let next = -1;
		if (e.key === 'ArrowDown') next = at < 0 ? 0 : (at + 1) % list.length;
		else if (e.key === 'ArrowUp') next = at < 0 ? list.length - 1 : (at - 1 + list.length) % list.length;
		else if (e.key === 'Home') next = 0;
		else if (e.key === 'End') next = list.length - 1;
		else if (e.key === 'Tab') {
			e.preventDefault();
			onclose();
			return;
		}
		if (next >= 0 && list[next]) {
			e.preventDefault();
			list[next].focus();
		}
	}

	function pointerover(e: PointerEvent) {
		const item = (e.target as HTMLElement).closest<HTMLElement>('[role^="menuitem"]');
		if (item && item.getAttribute('aria-disabled') !== 'true' && document.activeElement !== item)
			item.focus({ preventScroll: true });
	}

	function pointerleave() {
		if (panel?.contains(document.activeElement)) panel.focus({ preventScroll: true });
	}
</script>

<Popover {anchor} {side} {align} {gap} {flip} {onclose} role="menu" {label} bind:panel class={['menu', { quiet }, cls]}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="rows"
		style:min-width={width ? `${width}px` : undefined}
		onkeydown={keydown}
		onpointerover={pointerover}
		onpointerleave={pointerleave}
	>
		{@render children()}
	</div>
</Popover>

<style>
	.rows {
		display: flex;
		flex-direction: column;
		padding: var(--menu-padding-y) var(--menu-padding-x);
	}
</style>
