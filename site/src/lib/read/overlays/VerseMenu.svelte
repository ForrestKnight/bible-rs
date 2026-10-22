<!--
	The verse menu (verse_menu.rs): right-click a verse for the highlight swatches (1–6, 0 clears),
	note, bookmark, copy, share, compare and parallel.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { HIGHLIGHT_COLORS } from '../core';
	import { cmdText, isMac, shortcutText } from '../keyboard';
	import { Icon, Menu, MenuHeader, MenuItem, MenuSeparator } from '../ui';
	import { pointRect } from '../ui/float';

	const WIDTH = 264;

	const app = getApp();
	const at = app.verseMenu!;
	const anchor = pointRect(at.x, at.y);

	const refs = $derived(app.selectedRefs());
	const marks = $derived(refs.length ? app.marks(app.settings.translation, app.chapterIndex(refs[0])) : null);
	const first = $derived(refs.length ? marks?.get(refs[0].verse) : undefined);
	/** The colour all of the selection has (the chosen words, or the verses whole), if one. */
	const current = $derived(app.selectionColor());
	const keys = (id: string) => shortcutText(id, app.settings.shortcuts);

	const close = () => (app.verseMenu = null);
	const run = (a: Parameters<typeof app.dispatch>[0]) => {
		close();
		app.dispatch(a);
	};

	$effect(() => {
		if (!refs.length) close();
	});
</script>

<Menu
	{anchor}
	gap={0}
	flip={false}
	width={WIDTH}
	onclose={close}
	label="Verse"
	highlight={app.demo.menu === 'first' ? 0 : undefined}
>
	<MenuHeader label="Highlight" />
	<div class="swatches">
		{#each HIGHLIGHT_COLORS as c, k (c)}
			<button
				type="button"
				class={['swatch', { on: current === c }]}
				style:--swatch="var(--swatch-{c})"
				tabindex="-1"
				title="{app.highlightName(c)} ({k + 1})"
				aria-label="{app.highlightName(c)} highlight"
				aria-pressed={current === c}
				onclick={() => run({ type: 'highlight', color: current === c ? null : c })}
			>
				{#if current === c}<Icon name="check" size={14} />{/if}
			</button>
		{/each}
		<button
			type="button"
			class="clear"
			tabindex="-1"
			title="Remove highlight (0)"
			aria-label="Remove highlight"
			onclick={() => run({ type: 'highlight', color: null })}
		>
			<Icon name="slash-circle" size={20} />
		</button>
	</div>
	<MenuSeparator />
	<MenuItem
		icon="note"
		label={first?.note ? 'Edit Note…' : 'Add Note…'}
		shortcut={keys('note')}
		onclick={() => run({ type: 'editSelectedNote' })}
	/>
	<MenuItem
		icon="bookmark"
		label={first?.bookmark ? 'Remove Bookmark' : 'Bookmark'}
		shortcut={keys('bookmark')}
		onclick={() => run({ type: 'toggleBookmark' })}
	/>
	<MenuSeparator />
	<MenuItem icon="copy" label="Copy" shortcut={cmdText('C')} onclick={() => run({ type: 'copyText' })} />
	<MenuItem icon="quote" label="Copy with Reference" shortcut={keys('copy')} onclick={() => run({ type: 'copy' })} />
	<MenuItem icon="share" label="Share as Image…" onclick={() => run({ type: 'shareImage' })} />
	<MenuSeparator />
	<MenuItem icon="compare" label="Compare Translations" shortcut={keys('compare')} onclick={() => run({ type: 'compare' })} />
	<MenuItem icon="columns" label="Open in Parallel" shortcut={keys('parallel')} onclick={() => run({ type: 'toggleParallel' })} />
	<MenuSeparator />
	<MenuItem icon="text-cursor" label="Select Verse Range" shortcut={isMac ? '⇧-click' : 'Shift-click'} disabled onclick={() => {}} />
</Menu>

<style>
	/* 20 pt swatches 10 apart, 6 above and 8 below, from the text column. */
	.swatches {
		display: flex;
		gap: 10px;
		height: 34px;
		padding: 6px var(--menu-item-padding) 8px;
	}

	.swatch,
	.clear {
		position: relative;
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		isolation: isolate;
	}

	.swatch {
		background: var(--swatch);
		box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.18);
		color: #fff;
	}

	.swatch::before,
	.clear::before {
		content: '';
		position: absolute;
		inset: -3px;
		border-radius: 50%;
		z-index: -1;
	}

	.swatch:hover:not(.on)::before,
	.clear:hover::before {
		background: var(--fill-strong);
	}

	.swatch.on::before {
		inset: -3.5px;
		box-shadow: inset 0 0 0 1.5px var(--ink);
	}

	.clear {
		color: var(--ink-3);
	}
</style>
