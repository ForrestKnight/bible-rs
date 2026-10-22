<script lang="ts">
	import type { Book } from '../core';
	import { dominantColor, tint, type Mark } from './marks';
	import Ring from './Ring.svelte';

	interface Props {
		book: Book;
		/** The chapter being read (a chapter index), lit when it's in this book. */
		current: number;
		/** Each chapter's (index) marked verses. */
		activity: Map<number, Mark[]>;
		onpick: (chapter: number) => void;
	}

	let { book, current, activity, onpick }: Props = $props();
</script>

<!-- The open book's chapters: 24 pt chips in sevens, the current one solid accent. A chapter with
     highlights or notes gets a segmented ring (one segment per marked verse, in its colour) and
     its number on a disc tinted with its main highlight colour. -->
<div class="grid" role="group" aria-label="{book.name} chapters">
	{#each { length: book.chapters }, i (i)}
		{@const index = book.firstChapter + i}
		{@const here = index === current}
		{@const marks = activity.get(index)}
		<button
			type="button"
			class={['chip', { here, marked: !!marks?.length }]}
			aria-current={here || undefined}
			aria-label="{book.name} {i + 1}{marks?.length
				? `, ${marks.length} marked verse${marks.length === 1 ? '' : 's'}`
				: ''}"
			onclick={() => onpick(index)}
		>
			{#if marks?.length}
				<Ring
					{marks}
					size={24}
					fill={here ? 'var(--accent)' : tint(dominantColor(marks))}
					disc={8.5}
				/>
			{/if}
			<span class="n">{i + 1}</span>
		</button>
	{/each}
</div>

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(7, 24px);
		gap: 4px 8px;
		padding: 8px 0 8px 24px;
	}

	.chip {
		position: relative;
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		border-radius: 50%;
		color: var(--ink-2);
		font: var(--type-small);
		font-variant-numeric: tabular-nums;
		transition: background-color 100ms;
	}

	.chip > :global(.ring) {
		position: absolute;
		inset: 0;
	}

	.n {
		position: relative;
	}

	.chip:hover {
		background: var(--fill);
	}

	.chip:active {
		background: var(--fill-strong);
	}

	.chip.marked {
		color: var(--ink);
		font-weight: 600;
	}

	.chip.here {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	/* A marked current chapter keeps its accent to the disc, inside the ring. */
	.chip.here.marked {
		background: none;
	}
</style>
