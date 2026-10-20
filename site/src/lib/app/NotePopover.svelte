<script lang="ts">
	import Icon from './Icon.svelte';
	import { SWATCHES } from './VerseMenu.svelte';

	/** The note editor that opens beside a verse: its colour, the verse, and the note being written. */
	let {
		reference,
		excerpt,
		text,
		caret,
		pressed = false
	}: { reference: string; excerpt: string; text: string; caret: boolean; pressed?: boolean } = $props();
</script>

<div class="note">
	<div class="head">
		<Icon name="note" size={12} />
		<b>{reference}</b><span class="tr">KJV</span>
		<span class="swatches">
			{#each SWATCHES as color, i (color)}
				<span class={['swatch', { on: i === 0 }]} style:background={color}></span>
			{/each}
		</span>
	</div>
	<p class="excerpt">{excerpt}</p>
	<div class="field">
		{text}<span class="caret" style:opacity={caret ? 1 : 0}></span>
	</div>
	<div class="foot">
		<Icon name="trash" size={12} />
		<span class="saved">{text ? 'Saved · just now' : ''}</span>
		<span class={['done', { pressed }]}>Done</span>
	</div>
</div>

<style>
	.note {
		display: flex;
		flex-direction: column;
		gap: 9px;
		width: 270px;
		padding: 12px;
		border-radius: 12px;
		background: var(--popover);
		box-shadow: var(--popover-edge);
		color: var(--ink);
		font: 11px/14px var(--sans);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--accent);
	}

	.head b {
		color: var(--ink);
	}

	.tr {
		color: var(--ink-2);
		font-size: 10px;
	}

	.swatches {
		display: flex;
		gap: 5px;
		margin-left: auto;
	}

	.swatch {
		width: 11px;
		height: 11px;
		border-radius: 50%;
	}

	.swatch.on {
		box-shadow:
			0 0 0 1.5px var(--popover),
			0 0 0 2.5px var(--ink);
	}

	.excerpt {
		display: -webkit-box;
		margin: 0;
		padding-left: 9px;
		overflow: hidden;
		border-left: 2px solid #ebc24a;
		color: var(--ink-2);
		font: italic 10.5px/15px var(--serif);
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
	}

	.field {
		min-height: 62px;
		padding: 8px 9px;
		border-radius: 7px;
		background: var(--field);
		box-shadow:
			0 0 0 1px var(--accent),
			0 0 0 3.5px color-mix(in srgb, var(--accent) 25%, transparent);
		font-size: 11.5px;
		line-height: 16px;
	}

	.caret {
		display: inline-block;
		width: 1px;
		height: 13px;
		margin-left: 1px;
		background: var(--ink);
		vertical-align: -2px;
	}

	.foot {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--rubric);
	}

	.saved {
		flex: 1;
		color: var(--ink-3);
		font-size: 10px;
	}

	.done {
		padding: 3px 12px;
		border-radius: 999px;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
		transition: scale 0.12s;
	}

	.done.pressed {
		scale: 0.92;
	}
</style>
