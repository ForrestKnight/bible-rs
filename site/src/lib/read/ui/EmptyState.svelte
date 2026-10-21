<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';
	import Keycap from './Keycap.svelte';
	import type { IconName } from './icons';

	interface Props {
		glyph: IconName;
		tone?: 'accent' | 'danger' | 'quiet';
		title: string;
		/** Line breaks in it are kept. */
		body: string;
		/** The body's measure, narrowed to the space there is. */
		width?: number;
		/** Keycaps and what they do. */
		hints?: [string[], string][];
		/** Up to two buttons, side by side. */
		buttons?: Snippet;
		/** Below the buttons (the search's "Did you mean"). */
		after?: Snippet;
		/** Never higher than this from the top of the space. */
		minTop?: number;
	}

	let { glyph, tone = 'quiet', title, body, width = 320, hints = [], buttons, after, minTop = 24 }: Props = $props();
</script>

<!-- An empty state (pages/mod.rs `Empty`): a 56 pt circle with a glyph, a serif title, the body,
     then buttons and keycap hints, all centred on one axis and set a little above the middle. -->
<div class="space" style:--min-top="{minTop}px">
	<div class="empty">
		<div class={['circle', tone]}>
			<Icon name={glyph} size={28} heavy={tone === 'accent'} />
		</div>
		<h2 class="title">{title}</h2>
		<p class="body" style:max-width="{width}px">{body}</p>
		{#if buttons}<div class="buttons">{@render buttons()}</div>{/if}
		{#if hints.length}
			<div class="hints">
				{#each hints as [keys, what], i (i)}
					<span class="hint">
						{#each keys as k, j (j)}<Keycap label={k} />{/each}
						<span class="what">{what}</span>
					</span>
				{/each}
			</div>
		{/if}
		{#if after}{@render after()}{/if}
	</div>
</div>

<style>
	.space {
		display: grid;
		grid-template-rows: minmax(var(--min-top), 45fr) auto minmax(16px, 55fr);
		min-height: 100%;
	}

	.empty {
		grid-row: 2;
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
	}

	.circle {
		display: grid;
		place-items: center;
		width: 56px;
		height: 56px;
		border-radius: 50%;
		background: var(--fill);
		color: var(--ink-3);
	}

	.circle.accent {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		color: var(--accent);
	}

	.circle.danger {
		background: color-mix(in srgb, var(--danger) 12%, transparent);
		color: var(--danger);
	}

	.title {
		display: -webkit-box;
		margin: 12px 0 0;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		font: 400 20px/28px var(--font-reading);
		color: var(--ink);
	}

	/* 4 pt below the title as the Mac sets it, which leaves less of the leading above the text. */
	.body {
		margin: 2px 0 0;
		font: 400 13px/20px var(--font-ui);
		color: var(--ink-2);
		white-space: pre-line;
	}

	.buttons {
		display: flex;
		gap: 8px;
		margin-top: 16px;
	}

	.hints {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 8px 24px;
		margin-top: 24px;
	}

	.hint {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		height: 20px;
	}

	.what {
		margin-left: 4px;
		font: 400 12px/16px var(--font-ui);
		color: var(--ink-3);
	}
</style>
