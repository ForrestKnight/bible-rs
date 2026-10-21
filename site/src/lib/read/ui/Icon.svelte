<script lang="ts">
	import { ICONS, type IconDef, type IconName } from './icons';

	interface Props {
		name: IconName;
		/** The box, in px: 20 in the toolbar, 16 in rows and menus. */
		size?: number;
		/** 1.75-unit strokes, as 16 pt glyphs in tiles, menus and fields. */
		heavy?: boolean;
		/** Announced to screen readers; otherwise the icon is decorative. */
		label?: string;
		class?: string;
	}

	let { name, size = 20, heavy = false, label, class: cls }: Props = $props();

	const def: IconDef = $derived(ICONS[name]);
	const units = $derived(def.units ?? 20);
	const stroke = $derived(def.stroke ?? (heavy ? 1.75 : 1.5));
</script>

<svg
	class={['icon', cls]}
	width={size}
	height={size}
	viewBox="0 0 {units} {units}"
	stroke-width={stroke}
	role={label ? 'img' : undefined}
	aria-label={label}
	aria-hidden={label ? undefined : 'true'}
>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- constant markup from icons.ts -->
	{@html def.body}
</svg>

<style>
	.icon {
		flex-shrink: 0;
		display: block;
		fill: none;
		stroke: currentColor;
		stroke-linecap: round;
		stroke-linejoin: round;
		overflow: visible;
	}

	.icon :global(.f) {
		fill: currentColor;
		stroke: none;
	}

	.icon :global(.fs) {
		fill: currentColor;
	}

	.icon :global(.cut) {
		stroke: var(--icon-cut, var(--paper));
		stroke-width: 1.406;
	}
</style>
