<!--
	The Mac's overlay scroller (reader/mod.rs, paint_scrollbar): it spans the whole Bible, with a
	tick where the New Testament starts. It shows while scrolling and for a second after, or under
	the pointer; clicking or dragging goes to that place, and hovering names the chapter.
-->
<script lang="ts">
	import { CHAPTERS, chapterLabel } from '../core';

	interface Props {
		/** Where the reader is, 0–1 through the Bible. */
		frac: number;
		/** From the window's top to the bar's top. */
		top: number;
		/** When the reader last scrolled (performance.now()). */
		scrolledAt: number;
		/** The first New Testament chapter. */
		nt: number;
		onjump: (chapter: number, fraction: number) => void;
	}

	let { frac, top, scrolledAt, nt, onjump }: Props = $props();

	const THUMB = 36;
	let bar: HTMLDivElement;
	let hover = $state<{ y: number; label: string } | null>(null);
	let dragging = $state(false);
	let now = $state(0);

	// Visible for a second after scrolling.
	$effect(() => {
		void scrolledAt;
		now = performance.now();
		const t = setTimeout(() => (now = performance.now()), 1050);
		return () => clearTimeout(t);
	});

	const recent = $derived(scrolledAt > 0 && now - scrolledAt < 1000);
	const active = $derived(!!hover || dragging);

	function at(clientY: number) {
		const r = bar.getBoundingClientRect();
		const f = Math.min(0.9999, Math.max(0, (clientY - r.top) / r.height)) * CHAPTERS;
		return { chapter: Math.floor(f), fraction: f - Math.floor(f), y: clientY - r.top };
	}

	function onpointerdown(e: PointerEvent) {
		e.preventDefault();
		bar.setPointerCapture(e.pointerId);
		dragging = true;
		const p = at(e.clientY);
		onjump(p.chapter, p.fraction);
	}

	function onpointermove(e: PointerEvent) {
		const p = at(e.clientY);
		if (e.pointerType === 'mouse' || dragging) hover = { y: p.y, label: chapterLabel(p.chapter) };
		if (dragging) onjump(p.chapter, p.fraction);
	}

	function onpointerup(e: PointerEvent) {
		dragging = false;
		if (e.pointerType !== 'mouse') hover = null;
	}
</script>

<div
	class="bar"
	class:shown={active || recent}
	class:active
	bind:this={bar}
	style:top="{top}px"
	style:--thumb-y="calc({frac} * (100% - {THUMB}px))"
	style:--nt-y="{(nt / CHAPTERS) * 100}%"
	role="scrollbar"
	aria-controls="reader"
	aria-orientation="vertical"
	aria-valuemin={0}
	aria-valuemax={CHAPTERS}
	aria-valuenow={Math.round(frac * CHAPTERS)}
	tabindex="-1"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointerleave={() => !dragging && (hover = null)}
>
	<div class="tick"></div>
	<div class="thumb"></div>
	{#if hover}
		<div class="tip t-small" style:top="{hover.y}px">{hover.label}</div>
	{/if}
</div>

<style>
	.bar {
		position: absolute;
		right: 2px;
		bottom: 4px;
		width: 10px;
		border-radius: 6px;
		opacity: 0;
		transition: opacity 200ms;
		touch-action: none;
		z-index: 2;
	}

	.bar.shown {
		opacity: 1;
	}

	.bar.active {
		background: color-mix(in srgb, var(--fill) 80%, transparent);
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--fill) 80%, transparent);
	}

	.tick {
		position: absolute;
		left: 1px;
		right: 1px;
		top: var(--nt-y);
		height: 1px;
		background: var(--separator);
	}

	.thumb {
		position: absolute;
		left: 2px;
		right: 2px;
		top: var(--thumb-y);
		height: 36px;
		border-radius: 4px;
		background: color-mix(in srgb, var(--ink-3) 70%, transparent);
	}

	.active .thumb {
		background: var(--ink-2);
	}

	.tip {
		position: absolute;
		right: 16px;
		transform: translateY(-50%);
		padding: 4px 8px;
		border-radius: 6px;
		background: var(--raised);
		box-shadow: var(--shadow-popover);
		color: var(--ink);
		white-space: nowrap;
		pointer-events: none;
	}
</style>
