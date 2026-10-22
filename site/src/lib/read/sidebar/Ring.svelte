<script lang="ts">
	import { ringArcs, swatch, type Mark } from './marks';

	interface Props {
		marks: Mark[];
		/** Outer diameter, stroke width and the gap between segments, in points. */
		size: number;
		width?: number;
		gap?: number;
		/** Past this many marks the segments merge by colour. */
		cap?: number;
		/** A disc inside the ring (the chapter number's tint). */
		fill?: string;
		/** The disc's radius. */
		disc?: number;
	}

	let { marks, size, width = 2, gap = 2.5, cap = 12, fill, disc = 0 }: Props = $props();

	const c = $derived(size / 2);
	const r = $derived(size / 2 - width / 2);
	const arcs = $derived(ringArcs(marks, c, r, gap, cap));
</script>

<!-- A segmented ring: one segment per mark in its highlight's swatch (notes without a
     highlight in the accent), clockwise from the top, with clear gaps between. -->
<svg class="ring" width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
	{#if fill}<circle cx={c} cy={c} r={disc} {fill} />{/if}
	{#each arcs as arc, i (i)}
		{#if arc.d}
			<path d={arc.d} fill="none" stroke={swatch(arc.mark)} stroke-width={width} />
		{:else}
			<circle cx={c} cy={c} {r} fill="none" stroke={swatch(arc.mark)} stroke-width={width} />
		{/if}
	{/each}
</svg>

<style>
	.ring {
		display: block;
		flex-shrink: 0;
	}
</style>
