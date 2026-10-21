<!--
	Focus mode's chrome (app.rs `focus_chrome`): a reveal bar while the pointer is near the top (Exit
	Focus, text options, the shortcut), and a reading-progress hairline with what's left.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { getApp } from '../app.svelte';
	import { shortcutText } from '../keyboard';
	import { Capsule, ToolbarButton } from '../ui';
	import TextPopover from './TextPopover.svelte';

	/** The bar shows while the pointer is this near the top. */
	const REVEAL = 64;
	/** Reading speed for the time left, in words a minute. */
	const WPM = 230;

	const app = getApp();

	let near = $state(false);
	let within = $state(false);
	let mounted = $state(false);
	let textEl = $state<HTMLButtonElement>();

	const keys = $derived(mounted ? shortcutText('focus', app.settings.shortcuts) : '');
	const shown = $derived(near || within || app.textPopover || app.demo.reveal !== undefined);

	/** Each verse's number and words, in the chapter being read. */
	const verses = $derived(
		(app.verses(app.settings.translation, app.here.chapter) ?? []).map((v) => ({
			n: v.number,
			words: v.runs
				.map((r) => r.text)
				.join('')
				.split(/\s+/)
				.filter(Boolean).length
		}))
	);
	const progress = $derived.by(() => {
		const total = verses.length;
		if (!total) return null;
		const read = Math.min(app.furthest, total);
		const words = verses.filter((v) => v.n > read).reduce((sum, v) => sum + v.words, 0);
		const left = words === 0 ? 'the end of the chapter' : `about ${Math.ceil(words / WPM)} min left`;
		return { frac: read / total, text: `${read} of ${total} verses · ${left}` };
	});

	onMount(() => {
		mounted = true;
		const move = (e: PointerEvent) => (near = e.clientY < REVEAL);
		const leave = () => (near = false);
		window.addEventListener('pointermove', move);
		document.documentElement.addEventListener('pointerleave', leave);
		return () => {
			window.removeEventListener('pointermove', move);
			document.documentElement.removeEventListener('pointerleave', leave);
		};
	});
</script>

<div
	class={['reveal', { shown }]}
	role="toolbar"
	aria-label="Focus mode"
	tabindex="-1"
	onfocusin={() => (within = true)}
	onfocusout={(e) => (within = e.currentTarget.contains(e.relatedTarget as Node | null))}
>
	<Capsule dividers>
		<ToolbarButton
			icon="contract"
			label={keys ? `Exit Focus (${keys})` : 'Exit Focus'}
			onclick={() => app.dispatch({ type: 'toggleFocus' })}
		/>
		<ToolbarButton
			bind:el={textEl}
			icon="text-size"
			label="Text & theme"
			open={app.textPopover}
			onclick={() => app.dispatch({ type: 'textFormat' })}
		/>
	</Capsule>
	<span class="hint">Exit Focus {keys}</span>
</div>

{#if app.textPopover && textEl}
	<TextPopover anchor={textEl} onclose={() => (app.textPopover = false)} />
{/if}

<!-- The text scrolls under the progress line, which sits on the page colour. -->
<div class="band">
	{#if progress}
		<div class="track"><div class="done" style:width="{progress.frac * 100}%"></div></div>
		<p class="progress">{progress.text}</p>
	{/if}
</div>

<style>
	.reveal {
		position: absolute;
		top: 10px;
		left: 50%;
		display: flex;
		align-items: center;
		gap: 10px;
		transform: translateX(-50%);
		opacity: 0;
		pointer-events: none;
		transition: opacity var(--motion-base) var(--motion-easing);
		z-index: 4;
		outline: none;
	}

	.reveal:focus-within,
	.reveal.shown {
		opacity: 0.9;
		pointer-events: auto;
	}

	.hint {
		color: var(--ink-3);
		font: 500 12px/16px var(--font-ui);
		white-space: nowrap;
	}

	.band {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: 30px;
		background: var(--paper);
		pointer-events: none;
		z-index: 2;
	}

	.track {
		height: 2px;
		background: var(--fill);
	}

	.done {
		height: 100%;
		background: color-mix(in srgb, var(--accent) 50%, transparent);
	}

	.progress {
		margin: 0;
		position: absolute;
		left: 0;
		right: 0;
		top: 8px;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
		text-align: center;
		font-variant-numeric: tabular-nums;
	}
</style>
