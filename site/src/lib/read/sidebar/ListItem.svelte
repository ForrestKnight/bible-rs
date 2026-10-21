<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		reference: string;
		/** The translation's abbreviation, at the end of the reference's line. */
		trailing: string;
		/** A highlight's colour, as a bar down the leading edge. */
		bar?: string;
		note?: string;
		/** The verse, or `text` for marked-up text (a search hit's matches). */
		snippet?: string;
		text?: Snippet;
		/** The snippet's lines when it's `text`; a plain snippet gets two, one under a note. */
		lines?: number;
		selected?: boolean;
		onclick: (e: MouseEvent) => void;
		oncontextmenu?: (e: MouseEvent) => void;
		id?: string;
	}

	let { reference, trailing, bar, note, snippet, text, lines, selected = false, onclick, oncontextmenu, id }: Props =
		$props();

	const rows = $derived(lines ?? (note ? 1 : 2));
</script>

<!-- A study or search item: the reference with the translation at the end, an optional note
     (two lines), then the verse in the serif (two lines, one under a note), 12 × 10 pt in and
     radius 10. A highlight adds a 3 pt bar in its colour. -->
<button
	{id}
	type="button"
	class={['item', { selected, barred: bar }]}
	style:--bar={bar}
	aria-current={selected || undefined}
	{onclick}
	{oncontextmenu}
>
	<span class="head">
		<span class="reference">{reference}</span>
		<span class="trailing">{trailing}</span>
	</span>
	{#if note}<span class="note">{note}</span>{/if}
	<span class="snippet" style:-webkit-line-clamp={rows} style:line-clamp={rows}>
		{#if text}{@render text()}{:else}{snippet}{/if}
	</span>
</button>

<style>
	.item {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 4px;
		width: 100%;
		padding: 10px 12px;
		border-radius: 10px;
		text-align: left;
		transition: background-color 100ms;
	}

	.item:hover {
		background: var(--fill);
	}

	.item:active {
		background: var(--fill-strong);
	}

	.item.selected {
		background: var(--accent-soft);
	}

	.barred {
		padding-left: 23px;
	}

	.barred::before {
		content: '';
		position: absolute;
		top: 10px;
		bottom: 10px;
		left: 12px;
		width: 3px;
		border-radius: 2px;
		background: var(--bar);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 16px;
	}

	.reference {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		font: var(--type-ui-strong);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.selected .reference {
		color: var(--accent);
	}

	.trailing {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.note,
	.snippet {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		overflow: hidden;
		overflow-wrap: anywhere;
	}

	.note {
		-webkit-line-clamp: 2;
		line-clamp: 2;
		color: var(--ink);
		font: 400 13px/18px var(--font-ui);
	}

	.snippet {
		color: var(--ink-2);
		font: 400 14px/20px var(--font-reading);
	}
</style>
