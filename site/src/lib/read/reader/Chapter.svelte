<!--
	One chapter in a single column, typeset as the Mac reader does (crates/app/src/reader/layout.rs):
	the core breaks it into blocks with their spacing; this sets them in Literata. Spacing is in
	design points at a 20 pt reading size, scaled by --u.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { glyphsIn, type Glyph } from '../../objects/glyphs';
	import type { Block, ChapterContent, HighlightColor, Run, VerseMarks } from '../core';
	import Icon from '../ui/Icon.svelte';
	import { runClass, segments, washes, type Piece, type Wash } from './segments';

	interface Props {
		index: number;
		content: ChapterContent | null;
		/** "John 3", for a chapter still loading. */
		label: string;
		abbrev: string;
		marks: Map<number, VerseMarks>;
		selected: ReadonlySet<number>;
		flash: number | null;
		/** The verse whose note is being written: its marker carries the popover. */
		noteOpen: number | null;
		/** Focus mode: the head is one quiet line. */
		quiet: boolean;
		redLetters: boolean;
		justify: boolean;
		/** The chapter in the address: its verses are the targets of #v16 links. */
		anchors: boolean;
		/** Under the text (a live translation's credit line). */
		footer?: Snippet;
	}

	let {
		index,
		content,
		label,
		abbrev,
		marks,
		selected,
		flash,
		noteOpen,
		quiet,
		redLetters,
		justify,
		anchors,
		footer
	}: Props = $props();

	const PSALMS = 18;

	const blocks = $derived.by(() => {
		if (content?.status.type !== 'ready') return [];
		const seen = new Set<number>();
		return content.blocks.map((block) => ({ block, segs: segments(block, seen) }));
	});

	/** Verses that describe a modelled object (the Ark), by verse. */
	const glyphs = $derived(glyphsIn(index));
	/** Where each of those verses ends ("block:segment"): its glyph follows its last words. */
	const glyphAt = $derived.by(() => {
		const at = new Map<string, Glyph>();
		if (!glyphs) return at;
		const last = new Map<number, string>();
		blocks.forEach(({ segs }, i) => segs.forEach((s, j) => glyphs.has(s.verse) && last.set(s.verse, `${i}:${j}`)));
		for (const [verse, key] of last) at.set(key, glyphs.get(verse)!);
		return at;
	});

	const message = $derived(
		!content
			? 'Loading…'
			: content.status.type === 'missing'
				? 'This translation doesn’t include this chapter.'
				: content.status.type === 'failed'
					? `Couldn’t load this chapter: ${content.status.message}`
					: content.status.type === 'ready'
						? null
						: 'Loading…'
	);

	const book = $derived(content?.bookName ?? label.replace(/ \d+$/, ''));
	const number = $derived(content?.chapter ?? Number(label.match(/\d+$/)?.[0] ?? 1));
	const psalm = $derived(content ? content.book === PSALMS : label.startsWith('Psalm '));

	function kind(b: Block): string {
		const k = b.kind;
		switch (k.type) {
			case 'prose':
				return k.indent ? 'prose indent' : 'prose';
			case 'poetry':
				return 'poetry';
			case 'heading':
				return 'heading';
			case 'psalmHeading':
				return 'psalm-heading';
			case 'psalmTitle':
				return 'psalm-title';
			default:
				return k.type;
		}
	}

	const level = (b: Block) => (b.kind.type === 'poetry' ? b.kind.level : 0);
	const hl = (c: HighlightColor | null | undefined) => (c ? `var(--hl-${c})` : undefined);
	/** A verse's part of a block in one colour throughout takes the verse's own wash. */
	const whole = (ws: Wash[]) => (ws.length === 1 ? ws[0].color : null);
</script>

{#snippet run(r: Run)}{@const c = runClass(r, redLetters)}{#if c}<span class={c}>{r.text}</span
		>{:else}{r.text}{/if}{/snippet}

{#snippet piece(p: Piece)}<span class={p.cls || undefined} data-o={p.added ? undefined : p.offset}>{p.text}</span
	>{/snippet}

<!-- Parts of the verse in other colours wash only their words. -->
{#snippet text(ws: Wash[])}{#each ws as w, i (i)}{#if w.color && ws.length > 1}<span
				class="h"
				style:--hl={hl(w.color)}>{#each w.pieces as p, k (k)}{@render piece(p)}{/each}</span
			>{:else}{#each w.pieces as p, k (k)}{@render piece(p)}{/each}{/if}{/each}{/snippet}

<section class="chapter" class:justify data-chapter={index} class:message={!!message}>
	{#if quiet}
		<header class="head quiet">
			<p class="label">{psalm ? 'PSALM' : book.toUpperCase()} {number} · {abbrev}</p>
		</header>
	{:else if psalm}
		<header class="head psalm">
			<p class="label">PSALM {number}</p>
		</header>
	{:else if number === 1}
		<header class="head">
			<h1 class="display">{book}</h1>
			{#if content?.bookTitle}<p class="subtitle">{content.bookTitle}</p>{/if}
			<p class="label chapter-label">CHAPTER 1</p>
		</header>
	{:else}
		<header class="head">
			<p class="label">{book.toUpperCase()}</p>
			<h2 class="display numeral">{number}</h2>
		</header>
	{/if}
	{#if message}
		<p class="note-text">{message}</p>
	{:else}
		{#each blocks as { block, segs }, i (i)}
			<p
				class="blk {kind(block)}"
				class:first={i === 0}
				style:--level={level(block) || null}
				style:--space={block.spaceBefore}
			>{#if block.gutter}<span class="gutter" data-v={block.gutterVerse}>{block.gutter}</span
					>{/if}{#each segs as seg, j (j)}{#if seg.verse === 0}{#each seg.runs as r, k (k)}{@render run(
								r
							)}{/each}{:else}{@const m = marks.get(seg.verse)}{@const ws = washes(
								seg.runs,
								m?.highlights ?? [],
								redLetters
							)}<span
							class="seg"
							id={anchors && seg.start ? `v${seg.verse}` : undefined}
							data-v={seg.verse}
							data-vs={seg.start ? '' : undefined}
							>{#if seg.start && (m?.bookmark || m?.note || noteOpen === seg.verse)}<span class="markers"
									>{#if m?.bookmark}<button
											class="marker"
											data-marker="bookmark"
											data-v={seg.verse}
											title="Bookmark"
											aria-label="Bookmark"><Icon name="bookmark-fill" size={16} /></button
										>{/if}{#if m?.note || noteOpen === seg.verse}<button
											class="marker"
											class:open={noteOpen === seg.verse}
											data-marker="note"
											data-v={seg.verse}
											data-note-marker="{index}:{seg.verse}"
											title="Note"
											aria-label="Note"><Icon name="note" size={16} /></button
										>{/if}</span
								>{/if}{#if seg.number}<span class="vn">{seg.number.text}</span>{/if}<span
								class="v"
								class:sel={selected.has(seg.verse)}
								class:flash={flash === seg.verse}
								style:--hl={hl(whole(ws))}
								>{@render text(ws)}</span
							>{#if glyphAt.get(`${i}:${j}`)}{@const g = glyphAt.get(`${i}:${j}`)!}<a
									class="glyph"
									href={g.href}
									data-sveltekit-reload
									title={g.label}
									aria-label={g.label}><Icon name="cube" size={18} /></a
								>{/if}{seg.trail}</span
						>{/if}{/each}</p>
		{/each}
		{@render footer?.()}
	{/if}
</section>

<style>
	.chapter {
		padding-bottom: calc(80 * var(--u));
	}

	.chapter.message {
		padding-bottom: calc(6 * var(--reading-size));
	}

	.head {
		text-align: center;
		padding: calc(16 * var(--u)) 0 calc(24 * var(--u));
	}

	.head.psalm {
		text-align: left;
		padding: 0;
	}

	.head.quiet {
		padding: 0 0 calc(32px - 16 * var(--u));
	}

	.label {
		font-family: var(--font-ui);
		font-weight: 600;
		font-size: round(calc(11 * var(--u)), 1px);
		line-height: calc(16 * var(--u));
		letter-spacing: 0.12em;
		color: var(--accent);
	}

	.quiet .label {
		color: var(--ink-3);
	}

	.display {
		font-family: var(--font-reading);
		font-weight: 300;
		font-variation-settings: 'opsz' 48;
		font-size: round(calc(48 * var(--u)), 1px);
		line-height: calc(56 * var(--u));
		letter-spacing: -0.015em;
		color: var(--ink);
	}

	.numeral {
		margin-top: calc(4 * var(--u));
	}

	.subtitle {
		margin-top: calc(4 * var(--u));
		font-family: var(--font-reading);
		font-style: italic;
		font-size: round(calc(17 * var(--u)), 1px);
		line-height: calc(24 * var(--u));
		color: var(--ink-2);
	}

	.chapter-label {
		margin-top: calc(16 * var(--u));
	}

	.note-text {
		margin-top: 1em;
		font-family: var(--font-reading);
		font-style: italic;
		font-size: var(--reading-size);
		line-height: var(--lh);
		text-align: center;
		color: var(--ink-2);
	}

	.blk {
		margin-top: calc(var(--space) * var(--u));
		font-family: var(--font-reading);
		font-size: var(--reading-size);
		line-height: var(--lh);
		color: var(--ink);
		overflow-wrap: break-word;
	}

	.quiet + .blk.first {
		margin-top: max(calc(var(--space) * var(--u)), 20px);
	}

	.indent {
		text-indent: calc(32 * var(--u));
	}

	.justify .prose {
		text-align: justify;
	}

	.poetry {
		padding-left: calc(32 * var(--u) * var(--level));
		text-indent: calc(-32 * var(--u));
	}

	.right {
		text-align: right;
	}

	.center {
		text-align: center;
	}

	.heading {
		font-weight: 600;
		font-variation-settings: 'opsz' 16;
		font-size: round(calc(17 * var(--u)), 1px);
		line-height: calc(24 * var(--u));
	}

	.psalm-heading {
		font-variation-settings: 'opsz' 28;
		font-size: round(calc(28 * var(--u)), 1px);
		line-height: calc(36 * var(--u));
		letter-spacing: -0.01em;
	}

	.psalm-title,
	.speaker {
		font-style: italic;
		font-size: round(calc(17 * var(--u)), 1px);
		line-height: calc(24 * var(--u));
		color: var(--ink-2);
	}

	:is(.heading, .psalm-heading, .psalm-title, .speaker) :global(.sc) {
		letter-spacing: 0.03em;
	}

	.blk :global(.i) {
		font-style: italic;
	}

	.blk :global(.b) {
		font-weight: 600;
		font-variation-settings: 'opsz' 16;
	}

	.blk :global(.sc) {
		font-size: 0.75em;
		letter-spacing: 0.04em;
	}

	.blk :global(.red) {
		color: var(--rubric);
	}

	/* Inline verse numbers: 11 pt semibold in their own 20 pt line box at the top of the line. */
	.vn {
		font-family: var(--font-ui);
		font-weight: 600;
		font-style: normal;
		font-size: round(calc(11 * var(--u)), 1px);
		line-height: calc(20 * var(--u));
		vertical-align: top;
		color: var(--verse);
		text-indent: 0;
	}

	/* Verse-per-line and poetry numbers: 12 pt, right-aligned 12 pt before the column, centred on the first line. */
	.gutter {
		position: absolute;
		right: calc(100% + 12 * var(--u));
		height: var(--lh);
		line-height: var(--lh);
		font-family: var(--font-ui);
		font-weight: 600;
		font-style: normal;
		font-size: round(calc(12 * var(--u)), 1px);
		color: var(--verse);
		text-indent: 0;
		white-space: nowrap;
	}

	/*
	 * Highlights, the selection and the flash wash the verse's text: its 28 pt line box at 20 pt, from
	 * 1.1345 em above the baseline to 0.2655 em below (the font's content area less 0.0425 em each side).
	 */
	.v {
		--hl: transparent;
		--sel: transparent;
		background-image:
			linear-gradient(var(--fl), var(--fl)), linear-gradient(var(--sel), var(--sel)),
			linear-gradient(var(--hl), var(--hl));
		background-size: 100% calc(100% - 0.085em);
		background-position: 0 50%;
		background-repeat: no-repeat;
		border-radius: 4px;
		-webkit-box-decoration-break: clone;
		box-decoration-break: clone;
	}

	.v.sel {
		--sel: var(--selection);
	}

	/* Part of a verse in its own colour: the same wash over just its words (the selection's over it). */
	.h {
		background-image: linear-gradient(var(--sel), var(--sel)), linear-gradient(var(--hl), var(--hl));
		background-size: 100% calc(100% - 0.085em);
		background-position: 0 50%;
		background-repeat: no-repeat;
		border-radius: 4px;
		-webkit-box-decoration-break: clone;
		box-decoration-break: clone;
	}

	.v.flash {
		animation: flash 2.2s linear forwards;
	}

	@keyframes flash {
		0%,
		36% {
			--fl: var(--accent-soft);
		}
		100% {
			--fl: transparent;
		}
	}

	.seg {
		cursor: pointer;
	}

	/* Notes and bookmarks: 16 pt glyphs in 20 pt boxes, 32 pt right of the text, centred on the verse's first line. */
	.markers {
		position: absolute;
		left: calc(100% + 32 * var(--u));
		height: var(--lh);
		margin-top: -1px;
		/* Inline-level, so it stays on the line where the verse starts. */
		display: inline-flex;
		align-items: center;
		gap: 4px;
		text-indent: 0;
	}

	.marker {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: 6px;
		color: var(--accent);
		cursor: pointer;
	}

	.marker.open {
		background: var(--accent-soft);
	}

	/* An object modelled in 3D: an 18 pt cube after the verse, in the accent colour. */
	.glyph {
		display: inline-grid;
		place-items: center;
		width: 24px;
		height: 24px;
		margin-left: 2px;
		border-radius: 6px;
		vertical-align: -5px;
		color: var(--accent);
		text-indent: 0;
		transition: background-color var(--motion-fast);
	}

	.glyph:hover {
		background: var(--accent-soft);
	}
</style>
