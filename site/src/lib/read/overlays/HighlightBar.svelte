<!--
	The highlight swatches under a selection made by touch, which has no number keys or right-click:
	tap a colour to highlight the chosen words (or the tapped verses), the checked one or the slash
	to clear them. Mouse and keyboard use the verse menu and 1–6 / 0, as on the Mac.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { HIGHLIGHT_COLORS } from '../core';
	import { Icon, Popover } from '../ui';

	const app = getApp();

	const current = $derived(app.selectionColor());
	/** A new rectangle each time the selection changes, so the bar follows it. */
	const anchor = $derived.by(() => {
		void app.words;
		void app.selection;
		return () => app.reader?.selectionRect() ?? null;
	});
</script>

<Popover {anchor} side="bottom" align="center" gap={10} autofocus={false} label="Highlight" radius={20}>
	<div class="bar">
		{#each HIGHLIGHT_COLORS as c (c)}
			<button
				type="button"
				class={['swatch', { on: current === c }]}
				style:--swatch="var(--swatch-{c})"
				title={app.highlightName(c)}
				aria-label="{app.highlightName(c)} highlight"
				aria-pressed={current === c}
				onclick={() => app.dispatch({ type: 'highlight', color: current === c ? null : c })}
			>
				{#if current === c}<Icon name="check" size={14} />{/if}
			</button>
		{/each}
		<button
			type="button"
			class="clear"
			title="Remove highlight"
			aria-label="Remove highlight"
			onclick={() => app.dispatch({ type: 'highlight', color: null })}
		>
			<Icon name="slash-circle" size={22} />
		</button>
	</div>
</Popover>

<style>
	/* The verse menu's swatches, a little larger for a finger: 28 pt, 8 apart. */
	.bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
	}

	.swatch,
	.clear {
		position: relative;
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		isolation: isolate;
	}

	.swatch {
		background: var(--swatch);
		box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.18);
		color: #fff;
	}

	.swatch.on::before {
		content: '';
		position: absolute;
		inset: -3.5px;
		border-radius: 50%;
		box-shadow: inset 0 0 0 1.5px var(--ink);
		z-index: -1;
	}

	.clear {
		color: var(--ink-3);
	}
</style>
