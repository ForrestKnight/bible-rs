<!--
	The reading area: chapters one after another in a scroller, as on the Mac (reader/mod.rs).
	Only the chapters around the reading position are in the page; more are added as it scrolls,
	keeping what's on screen still. Clicking a verse selects it, shift-click extends, double-click
	writes a note, right-click opens the verse menu.
-->
<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { getApp, type ReaderHandle } from '../app.svelte';
	import { CHAPTERS, chapterLabel, type VerseRef } from '../core';
	import Chapter from './Chapter.svelte';
	import Parallel, { type Column } from './Parallel.svelte';
	import Scrollbar from './Scrollbar.svelte';

	interface Props {
		/** From the window's top to where a chapter scrolled to its start begins. */
		top: number;
		/** From the window's top to the bottom of the chrome over the text. */
		chrome: number;
		/** A page covers the reader. */
		covered: boolean;
	}

	let { top, chrome, covered }: Props = $props();

	const app = getApp();
	let scroller: HTMLDivElement;
	let column: HTMLDivElement;

	let first = $state(app.here.chapter);
	let last = $state(app.here.chapter);
	const indices = $derived(Array.from({ length: last - first + 1 }, (_, i) => first + i));

	const cols = $derived(app.columns());
	const translation = $derived(cols[0]);
	const parallel = $derived(cols.length > 1);
	const typo = $derived(app.settings.typo);
	const abbrev = $derived(app.info(translation)?.abbrev ?? translation.toUpperCase());
	const ready = $derived(app.loaded.has(translation));

	let flash = $state<{ chapter: number; verse: number } | null>(null);
	let flashTimer: ReturnType<typeof setTimeout> | undefined;
	let pending: { chapter: number; verse: number | undefined; flash: boolean } | null = null;
	/** The verse at the top line: how far into it, or how far above it (a head, a gap). */
	let anchor: { chapter: number; verse: number; frac: number; above: number } | null = null;
	/** The top line the page is laid out for: before an update reaches the page, `top` may
	 *  already be the next one (focus mode, the search band). */
	let laidTop = untrack(() => top);
	$effect(() => {
		laidTop = top;
	});
	let scrolledAt = $state(0);

	/** The verses selected in a chapter. */
	function selectedIn(index: number): Set<number> {
		const out = new Set<number>();
		for (const v of app.selectedRefs()) if (app.chapterIndex(v) === index) out.add(v.verse);
		return out;
	}

	function columnsFor(index: number): Column[] {
		return cols.map((id) => {
			const verses = app.verses(id, index);
			return {
				id,
				verses,
				message: !app.loaded.has(id)
					? 'Loading…'
					: verses && verses.length === 0
						? 'This translation doesn’t include this chapter.'
						: null
			};
		});
	}

	function parallelLabel(index: number): string {
		return app.core ? chapterLabel(index).toUpperCase() : '';
	}

	// ------------------------------------------------------------------ geometry

	const sectionEl = (index: number) =>
		column?.querySelector<HTMLElement>(`section[data-chapter="${index}"]`) ?? null;

	/** An element's top in the scroller's content. */
	function contentTop(el: Element): number {
		return el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
	}

	function verseEls(section: Element, verse: number): HTMLElement[] {
		return [...section.querySelectorAll<HTMLElement>(`[data-v="${verse}"]`)];
	}

	/** A verse's offset from its chapter's start: from its paragraph's start when that is only a few lines up. */
	function verseOffset(section: HTMLElement, verse: number): number {
		const el = verseEls(section, verse)[0];
		if (!el) return 0;
		const em = typo.size;
		const sTop = contentTop(section);
		const vTop = (el.getClientRects()[0]?.top ?? el.getBoundingClientRect().top) - section.getBoundingClientRect().top;
		const block = el.closest('.blk, .row');
		if (block) {
			const bTop = contentTop(block) - sTop - em * 0.5;
			if (bTop <= vTop + 1 && vTop - bTop < scroller.clientHeight * 0.35) return Math.max(0, bTop);
		}
		return Math.max(0, vTop - em * 2.2);
	}

	// ------------------------------------------------------------------ jumping

	async function jump(chapter: number, verse: number | undefined, fl: boolean) {
		chapter = Math.max(0, Math.min(CHAPTERS - 1, chapter));
		pending = { chapter, verse, flash: fl };
		app.here = { chapter, verse: verse ?? 1 };
		if (chapter < first || chapter > last) {
			first = chapter;
			last = chapter;
		}
		await tick();
		place();
	}

	/** Scrolls to the pending jump once its chapter has text. */
	function place() {
		const p = pending;
		const section = p && sectionEl(p.chapter);
		if (!p || !section) return;
		const hasText = parallel ? app.loaded.has(translation) : app.content(translation, p.chapter) !== null;
		const offset = p.verse && p.verse > 1 && hasText ? verseOffset(section, p.verse) : 0;
		const target = contentTop(section) - top + offset;
		// Near a chapter's end there may not be enough text below to scroll that far yet.
		if (target > scroller.scrollHeight - scroller.clientHeight + 1 && last < CHAPTERS - 1) {
			last++;
			void tick().then(place);
			return;
		}
		stopScroll();
		scroller.scrollTop = target;
		if (hasText || !p.verse || p.verse <= 1) pending = null;
		if (hasText && p.flash && p.verse) {
			flash = { chapter: p.chapter, verse: p.verse };
			clearTimeout(flashTimer);
			flashTimer = setTimeout(() => (flash = null), 2300);
		}
		locate();
		remember();
		measureUnder();
		schedule();
	}

	// Text arriving (a translation loading) places a pending jump, or puts the verse that was at
	// the top back there.
	$effect(() => {
		void app.loaded.size;
		void cols;
		untrack(() =>
			tick().then(() => {
				place();
				restore();
			})
		);
	});

	// ------------------------------------------------------------------ keeping the place

	/** Remembers the verse at the top line, so a relayout can keep it there. */
	function remember() {
		// Until the translation loads its chapters are placeholders: keep the verse from before.
		if (!scroller || !column || !app.loaded.has(translation)) return;
		const line = scroller.getBoundingClientRect().top + laidTop;
		for (const section of column.querySelectorAll<HTMLElement>('section[data-chapter]')) {
			const r = section.getBoundingClientRect();
			if (r.bottom <= line) continue;
			const chapter = Number(section.dataset.chapter);
			// A chapter starting at the line stays at its start, whatever its head becomes.
			if (r.top >= line - 1) {
				anchor = { chapter, verse: 0, frac: 0, above: r.top - line };
				return;
			}
			for (const el of section.querySelectorAll<HTMLElement>('[data-vs]')) {
				const vr = (el.closest('.row') ?? el).getBoundingClientRect();
				if (vr.bottom <= line) continue;
				const verse = Number(el.dataset.v);
				anchor =
					vr.top >= line
						? { chapter, verse, frac: 0, above: vr.top - line }
						: { chapter, verse, frac: (line - vr.top) / Math.max(1, vr.height), above: 0 };
				return;
			}
			anchor = { chapter, verse: 0, frac: 0, above: r.top - line };
			return;
		}
	}

	/** Whether any text is beneath the chrome (the toolbar then takes its material). */
	function measureUnder() {
		if (!scroller || !column) return;
		const sr = scroller.getBoundingClientRect();
		const line = sr.top + chrome;
		let under = false;
		for (const section of column.querySelectorAll<HTMLElement>('section[data-chapter]')) {
			const r = section.getBoundingClientRect();
			if (r.bottom <= sr.top) continue;
			if (r.top >= line) break;
			for (const el of section.querySelectorAll<HTMLElement>('p, h1, h2')) {
				const er = el.getBoundingClientRect();
				if (er.top >= line) break;
				if (er.bottom > sr.top) {
					under = true;
					break;
				}
			}
			if (under) break;
		}
		if (app.underToolbar !== under) app.underToolbar = under;
	}

	/** Puts the remembered verse back at the top line after a relayout. */
	function restore() {
		const a = anchor;
		if (!a || pending) return;
		const section = sectionEl(a.chapter);
		if (!section) return;
		const line = scroller.getBoundingClientRect().top + top;
		const el = a.verse ? section.querySelector<HTMLElement>(`[data-vs][data-v="${a.verse}"]`) : section;
		if (!el) return;
		const r = (el.closest('.row') ?? el).getBoundingClientRect();
		const target = scroller.scrollTop + r.top + a.frac * r.height - a.above - line;
		// Chapters that got shorter (a translation without them) may leave too little text to
		// scroll that far: add some, then place it.
		if (target > scroller.scrollHeight - scroller.clientHeight + 1 && last < CHAPTERS - 1) {
			last++;
			void tick().then(restore);
			return;
		}
		if (target < 0 && first > 0) {
			first--;
			void tick().then(restore);
			return;
		}
		scroller.scrollTop = target;
	}

	// A change in layout (typography, translation, columns) keeps the verse at the top in place.
	$effect.pre(() => {
		void [typo.size, typo.lineHeight, typo.mode, typo.measure, typo.headings, typo.verseNumbers, typo.justify];
		void cols.join();
		void app.focus;
		untrack(() => {
			if (scroller) remember();
		});
	});

	$effect(() => {
		void [typo.size, typo.lineHeight, typo.mode, typo.measure, typo.headings, typo.verseNumbers, typo.justify];
		void cols.join();
		void app.focus;
		// After the rest of the update: the reading size reaches <html> in the shell's effect.
		untrack(() =>
			tick().then(() => {
				if (scroller) {
					restore();
					schedule();
				}
			})
		);
	});

	// ------------------------------------------------------------------ the chapter window

	let frame = 0;
	function schedule() {
		if (!frame) frame = requestAnimationFrame(step);
	}

	/** One step a frame: add a chapter where the text runs out, drop ones far away. */
	async function step() {
		frame = 0;
		if (!scroller || !app.core || !ready || pending) return;
		const vh = scroller.clientHeight;
		const st = scroller.scrollTop;
		const below = scroller.scrollHeight - (st + vh);
		let changed = false;
		if (below < vh * 1.5 && last < CHAPTERS - 1) {
			last++;
			changed = true;
		} else if (st < vh && first > 0) {
			await keepingPlace(() => first--);
			changed = true;
		} else {
			const firstEl = sectionEl(first);
			const lastEl = sectionEl(last);
			if (first < last && firstEl && firstEl.getBoundingClientRect().bottom < scroller.getBoundingClientRect().top - vh * 3) {
				await keepingPlace(() => first++);
				changed = true;
			} else if (first < last && lastEl && contentTop(lastEl) > st + vh * 4) {
				last--;
				changed = true;
			}
		}
		if (changed) {
			await tick();
			locate();
			schedule();
		}
	}

	/** Changes the chapters above the view without moving what's on screen. */
	async function keepingPlace(change: () => void) {
		const ref = sectionEl(app.here.chapter) ?? sectionEl(last);
		const before = ref?.getBoundingClientRect().top ?? 0;
		change();
		await tick();
		if (ref?.isConnected) scroller.scrollTop += ref.getBoundingClientRect().top - before;
	}

	// ------------------------------------------------------------------ where the reader is

	/** The chapter `app.furthest` counts verses in. */
	let furthestIn = -1;

	/** The first verse whose last line is below the reading line (22% down), and the furthest in view. */
	function locate() {
		if (!scroller || pending) return;
		const sr = scroller.getBoundingClientRect();
		const line = sr.top + sr.height * 0.22;
		for (const section of column.querySelectorAll<HTMLElement>('section[data-chapter]')) {
			const r = section.getBoundingClientRect();
			if (r.bottom <= line) continue;
			const chapter = Number(section.dataset.chapter);
			const starts = [...section.querySelectorAll<HTMLElement>('[data-vs]')];
			let verse = starts.length ? Number(starts[0].dataset.v) : 1;
			for (const el of section.querySelectorAll<HTMLElement>('[data-v]')) {
				if (el.getBoundingClientRect().bottom > line) {
					verse = Number(el.dataset.v);
					break;
				}
			}
			if (app.here.chapter !== chapter || app.here.verse !== verse) app.here = { chapter, verse };
			let furthest = 0;
			const bottom = sr.bottom - typo.size;
			for (const el of starts) {
				if (el.getBoundingClientRect().top < bottom) furthest = Number(el.dataset.v);
				else break;
			}
			if (furthest > app.furthest || furthestIn !== chapter) {
				app.furthest = furthest;
				furthestIn = chapter;
			}
			return;
		}
	}

	/** Where the reader is through the whole Bible (the scrollbar's thumb). */
	let scrollFrac = $state(0);

	function measureFrac() {
		const section = sectionEl(app.here.chapter);
		if (!section || !scroller) return;
		const r = section.getBoundingClientRect();
		const into = scroller.getBoundingClientRect().top + top - r.top;
		scrollFrac = (app.here.chapter + Math.min(1, Math.max(0, into / Math.max(1, r.height)))) / CHAPTERS;
	}

	async function barJump(chapter: number, fraction: number) {
		await jump(chapter, undefined, false);
		const s = sectionEl(chapter);
		if (s) scroller.scrollTop += fraction * s.offsetHeight;
		measureFrac();
	}

	let locating = 0;
	function onscroll() {
		scrolledAt = performance.now();
		if (!locating)
			locating = requestAnimationFrame(() => {
				locating = 0;
				locate();
				remember();
				measureFrac();
				measureUnder();
				schedule();
			});
	}

	// ------------------------------------------------------------------ smooth scrolling (keys)

	let remaining = 0;
	let scrollFrame = 0;
	const reduceMotion = () =>
		app.settings.reduceMotion || matchMedia('(prefers-reduced-motion: reduce)').matches;

	function scrollBy(dy: number) {
		if (reduceMotion()) {
			scroller.scrollTop += dy;
			return;
		}
		remaining += dy;
		if (!scrollFrame) scrollFrame = requestAnimationFrame(scrollStep);
	}

	function scrollStep() {
		let s = remaining * 0.28;
		if (Math.abs(s) < 0.5) s = remaining;
		scroller.scrollTop += s;
		remaining -= s;
		scrollFrame = Math.abs(remaining) > 0.5 ? requestAnimationFrame(scrollStep) : 0;
		if (!scrollFrame) remaining = 0;
	}

	function stopScroll() {
		cancelAnimationFrame(scrollFrame);
		scrollFrame = 0;
		remaining = 0;
	}

	// ------------------------------------------------------------------ pointer

	function caretElement(x: number, y: number): Element | null {
		const d = document as Document & {
			caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node } | null;
		};
		const node = d.caretPositionFromPoint
			? d.caretPositionFromPoint(x, y)?.offsetNode
			: document.caretRangeFromPoint?.(x, y)?.startContainer;
		if (!node) return null;
		return node instanceof Element ? node : node.parentElement;
	}

	/** The verse under the pointer: within its block (or row), give or take 8 × 2 pt. */
	function verseAt(e: MouseEvent): VerseRef | null {
		const target = e.target as Element;
		let el = target.closest('[data-v]');
		const row = target.closest<HTMLElement>('.row');
		if (row) return refIn(row, Number(row.dataset.row));
		if (!el) el = caretElement(e.clientX, e.clientY)?.closest('[data-v]') ?? null;
		if (!el) return null;
		const block = el.closest('.blk');
		if (block) {
			const r = block.getBoundingClientRect();
			if (e.clientX < r.left - 8 || e.clientX > r.right + 8 || e.clientY < r.top - 2 || e.clientY > r.bottom + 2)
				return null;
		}
		return refIn(el, Number((el as HTMLElement).dataset.v));
	}

	function refIn(el: Element, verse: number): VerseRef | null {
		const section = el.closest<HTMLElement>('section[data-chapter]');
		return section ? app.refAt(Number(section.dataset.chapter), verse) : null;
	}

	function onclick(e: MouseEvent) {
		if (e.button !== 0 || !getSelection()?.isCollapsed) return;
		const marker = (e.target as Element).closest<HTMLElement>('[data-marker]');
		if (marker) {
			const v = refIn(marker, Number(marker.dataset.v));
			if (v) app.dispatch({ type: 'editNote', verse: v });
			return;
		}
		const v = verseAt(e);
		if (v) app.select(v, e.shiftKey);
		else app.selection = null;
	}

	function ondblclick(e: MouseEvent) {
		const v = verseAt(e);
		if (!v) return;
		getSelection()?.removeAllRanges();
		app.setSelection(v);
		app.dispatch({ type: 'editNote', verse: v });
	}

	function oncontextmenu(e: MouseEvent) {
		const v = verseAt(e);
		if (!v) return;
		e.preventDefault();
		if (!app.selectedRefs().some((s) => s.book === v.book && s.chapter === v.chapter && s.verse === v.verse))
			app.setSelection(v);
		app.dispatch({ type: 'openVerseMenu', x: e.clientX, y: e.clientY });
	}

	function onmousedown(e: MouseEvent) {
		// Shift-click extends the verse selection, not the text selection.
		if (e.shiftKey && verseAt(e)) e.preventDefault();
	}

	// ------------------------------------------------------------------ the handle

	const handle: ReaderHandle = {
		jump: (c, v, f) => void jump(c, v, f),
		scrollBy,
		page: () => Math.max(scroller.clientHeight - chrome - typo.size * 3, typo.size * 4),
		offsetInChapter() {
			const section = sectionEl(app.here.chapter);
			if (!section) return 0;
			return scroller.getBoundingClientRect().top + top - section.getBoundingClientRect().top;
		},
		ensureVisible(v) {
			const index = app.chapterIndex(v);
			const section = sectionEl(index);
			const el = section && (section.querySelector(`.row[data-row="${v.verse}"]`) ?? verseEls(section, v.verse)[0]);
			if (!el) {
				void jump(index, v.verse, false);
				return;
			}
			const sr = scroller.getBoundingClientRect();
			const r = el.getBoundingClientRect();
			const margin = typo.size * 3;
			if (r.top < sr.top + chrome + margin) scrollBy(r.top - sr.top - chrome - margin);
			else if (r.bottom > sr.bottom - margin) scrollBy(r.bottom - sr.bottom + margin);
		},
		selectionRect() {
			const els = column.querySelectorAll('.v.sel, .row.sel');
			let rect: DOMRect | null = null;
			for (const el of els) {
				for (const r of el.getClientRects()) {
					rect = rect ? union(rect, r) : DOMRect.fromRect(r);
				}
			}
			return rect;
		},
		markerRect(v) {
			const el = column.querySelector(`[data-note-marker="${app.chapterIndex(v)}:${v.verse}"]`);
			return el?.getBoundingClientRect() ?? null;
		}
	};

	function union(a: DOMRect, b: DOMRect): DOMRect {
		const x = Math.min(a.left, b.left);
		const y = Math.min(a.top, b.top);
		return new DOMRect(x, y, Math.max(a.right, b.right) - x, Math.max(a.bottom, b.bottom) - y);
	}

	onMount(() => {
		app.reader = handle;
		const ro = new ResizeObserver(() => {
			restore();
			schedule();
		});
		ro.observe(scroller);
		const fonts = () => {
			restore();
			schedule();
		};
		document.fonts?.addEventListener('loadingdone', fonts);
		return () => {
			ro.disconnect();
			document.fonts?.removeEventListener('loadingdone', fonts);
			if (app.reader === handle) app.reader = null;
			cancelAnimationFrame(frame);
			cancelAnimationFrame(scrollFrame);
		};
	});

	// The core arriving fills in the neighbouring chapters.
	$effect(() => {
		if (ready && app.core) untrack(schedule);
	});
</script>

<div
	class="scroller"
	bind:this={scroller}
	style:padding-top="{top}px"
	{onscroll}
	inert={covered}
	aria-hidden={covered}
>
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
	<div
		class="column"
		class:parallel
		bind:this={column}
		style:--n={cols.length}
		{onclick}
		{ondblclick}
		{oncontextmenu}
		{onmousedown}
		role="document"
	>
		{#each indices as index (index)}
			{#if parallel}
				<Parallel
					{index}
					label={parallelLabel(index)}
					columns={columnsFor(index)}
					marks={app.marks(translation, index)}
					selected={selectedIn(index)}
					flash={flash?.chapter === index ? flash.verse : null}
					redLetters={typo.redLetters}
					verseNumbers={typo.verseNumbers}
				/>
			{:else}
				<Chapter
					{index}
					content={app.content(translation, index)}
					label={app.core ? chapterLabel(index) : ''}
					{abbrev}
					marks={app.marks(translation, index)}
					selected={selectedIn(index)}
					flash={flash?.chapter === index ? flash.verse : null}
					noteOpen={app.note && app.chapterIndex(app.note) === index ? app.note.verse : null}
					quiet={app.focus}
					redLetters={typo.redLetters}
					justify={typo.justify}
				/>
			{/if}
		{/each}
	</div>
</div>
<Scrollbar frac={scrollFrac} top={chrome + 4} {scrolledAt} nt={app.core?.books[39]?.firstChapter ?? 929} onjump={barJump} />

<style>
	.scroller {
		position: absolute;
		inset: 0;
		overflow-x: hidden;
		overflow-y: auto;
		overflow-anchor: none;
		overscroll-behavior: contain;
		container-type: inline-size;
		scrollbar-width: none;
		outline: none;
	}

	.scroller::-webkit-scrollbar {
		display: none;
	}

	.column {
		--margin: clamp(80px, 6cqw, 96px);
		position: relative;
		width: min(var(--reading-measure), calc(100% - 2 * var(--margin)));
		margin: 0 auto;
		font-size: var(--reading-size);
		user-select: text;
		-webkit-user-select: text;
	}

	@container (max-width: 600px) {
		.column {
			--margin: 28px;
		}
	}

	/* Parallel: rows padded 8 pt inside a 24 pt left and 16 pt right margin, columns 32 pt apart, up to 20 em each. */
	.column.parallel {
		--col-gap: round(calc(32 * var(--u)), 1px);
		--col-w: min(
			20em,
			calc((100cqw - 40 * var(--u) - 16px - var(--col-gap) * (var(--n) - 1)) / var(--n))
		);
		--total: calc(var(--col-w) * var(--n) + var(--col-gap) * (var(--n) - 1));
		width: calc(var(--total) + 16px);
		margin: 0 0 0 max(calc(24 * var(--u)), calc((100cqw - var(--total)) / 2 - 8px));
	}
</style>
