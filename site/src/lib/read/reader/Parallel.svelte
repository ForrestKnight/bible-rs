<!--
	One chapter as verse-aligned rows across translations (the Mac's layout_parallel, frame 07): a
	centred label, then a row per verse, each column a 24 pt number lane and the verse in Literata at
	17/28 (at a 20 pt reading size). A selected verse is marked as a whole row.
-->
<script lang="ts" module>
	import type { VerseContent } from '../core';

	export interface Column {
		id: string;
		verses: VerseContent[] | null;
		/** Why a column has no text ("Loading…", missing chapter). */
		message: string | null;
	}
</script>

<script lang="ts">
	import type { HighlightColor, Run, VerseMarks } from '../core';
	import { runClass } from './segments';

	interface Props {
		index: number;
		/** "JOHN 3", "PSALM 23". */
		label: string;
		columns: Column[];
		marks: Map<number, VerseMarks>;
		selected: ReadonlySet<number>;
		flash: number | null;
		redLetters: boolean;
		verseNumbers: boolean;
	}

	let { index, label, columns, marks, selected, flash, redLetters, verseNumbers }: Props = $props();

	const rows = $derived.by(() => {
		const numbers = new Set<number>();
		for (const c of columns) for (const v of c.verses ?? []) numbers.add(v.number);
		const byColumn = columns.map((c) => new Map((c.verses ?? []).map((v) => [v.number, v])));
		return [...numbers].sort((a, b) => a - b).map((n) => ({ n, cells: byColumn.map((m) => m.get(n) ?? null) }));
	});

	const hl = (c: HighlightColor | null | undefined) => (c ? `var(--hl-${c})` : undefined);
</script>

{#snippet run(r: Run)}{@const c = runClass(r, redLetters)}{#if c}<span class={c}>{r.text}</span
		>{:else}{r.text}{/if}{/snippet}

<section class="chapter" data-chapter={index} style:--n={columns.length}>
	<p class="label">{label}</p>
	{#if columns.some((c) => c.message)}
		<div class="messages">
			{#each columns as c (c.id)}
				<p class="message">{c.message ?? ''}</p>
			{/each}
		</div>
	{/if}
	{#each rows as row (row.n)}
		{@const m = marks.get(row.n)}
		<div class="row" class:sel={selected.has(row.n)} data-row={row.n}>
			{#each row.cells as v, ci (ci)}
				<div class="cell">
					{#if v}
						{#if verseNumbers}<span class="num" data-v={v.number}>{v.number}</span>{/if}
						<p class="text" data-v={v.number} data-vs={ci === 0 ? '' : undefined}>{#if v.title && v.number === 1}<span
									class="title">{v.title}</span
								><br />{/if}<span
								class="v seg"
								class:flash={flash === v.number}
								style:--hl={hl(m?.highlight)}>{#each v.runs as r, k (k)}{@render run(r)}{/each}</span
							></p>
					{/if}
				</div>
			{/each}
		</div>
	{/each}
</section>

<style>
	.chapter {
		--size: round(calc(17 * var(--u)), 1px);
		--row-lh: round(calc(28 * var(--u)), 1px);
		padding-bottom: calc(80 * var(--u));
	}

	.label {
		font-family: var(--font-ui);
		font-weight: 600;
		font-size: round(calc(11 * var(--u)), 1px);
		line-height: calc(16 * var(--u));
		letter-spacing: 0.12em;
		color: var(--accent);
		text-align: center;
		margin-bottom: calc(16 * var(--u));
	}

	.row,
	.messages {
		position: relative;
		display: grid;
		grid-template-columns: repeat(var(--n), var(--col-w));
		column-gap: var(--col-gap);
		padding: 6px 8px;
		cursor: pointer;
	}

	.row.sel::before {
		content: '';
		position: absolute;
		inset: calc(-0.2 * var(--size)) 0 0;
		border-radius: 10px;
		background: var(--selection);
		z-index: -1;
	}

	.cell {
		position: relative;
		padding-left: 24px;
		min-width: 0;
	}

	.num {
		position: absolute;
		left: 0;
		top: 0;
		line-height: var(--row-lh);
		font-family: var(--font-ui);
		font-weight: 600;
		font-size: round(calc(11 * var(--u)), 1px);
		color: var(--verse);
	}

	.text,
	.message {
		font-family: var(--font-reading);
		font-size: var(--size);
		line-height: var(--row-lh);
		color: var(--ink);
		overflow-wrap: break-word;
	}

	.message {
		padding-left: 24px;
		font-style: italic;
		color: var(--ink-2);
		min-height: calc(3 * var(--row-lh));
	}

	.title {
		font-style: italic;
		font-size: round(calc(var(--size) * 0.88), 1px);
		color: var(--ink-2);
	}

	.text :global(.i) {
		font-style: italic;
	}

	.text :global(.b) {
		font-weight: 600;
		font-variation-settings: 'opsz' 16;
	}

	.text :global(.sc) {
		font-size: 0.75em;
		letter-spacing: 0.04em;
	}

	.text :global(.red) {
		color: var(--rubric);
	}

	.v {
		--hl: transparent;
		background-image: linear-gradient(var(--fl), var(--fl)), linear-gradient(var(--hl), var(--hl));
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
</style>
