<!--
	The note popover (note_popover.rs): a verse's note, written in a popover hanging from its margin
	marker. The reference and highlight colour, the verse quoted, the editor, then delete, the saved
	state and Done. It saves a moment after typing stops, and on closing.
-->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { getApp, now } from '../app.svelte';
	import { HIGHLIGHT_COLORS } from '../core';
	import { isMac } from '../keyboard';
	import { ago } from '../time';
	import { Icon, Popover } from '../ui';

	const WIDTH = 344;
	/** The beak's tip sits this far from the popover's left edge, unless that's off screen. */
	const TIP = 173;
	const BEAK = 9;
	const AUTOSAVE_MS = 1200;

	const app = getApp();
	const verse = app.note!;
	const existing = untrack(() => app.noteFor(verse));

	let note = $state(existing?.body ?? '');
	let saved = $state(existing?.body ?? '');
	let savedAt = $state<number | null>(existing?.updated ?? null);
	let clock = $state(now());
	let deleted = false;
	/** The beak's tip, from the popover's left edge. */
	let tip = $state(TIP);
	let timer: ReturnType<typeof setTimeout> | undefined;
	let editor = $state<HTMLTextAreaElement>();

	const info = $derived(app.info(app.settings.translation));
	const reference = $derived(`${app.core?.books[verse.book]?.name ?? ''} ${verse.chapter}:${verse.verse}`);
	const quote = $derived(
		app.loaded.has(app.settings.translation)
			? (app.core?.verse(app.settings.translation, verse)?.runs.map((r) => r.text).join('') ?? '')
			: ''
	);
	const colour = $derived(app.marks(app.settings.translation, app.chapterIndex(verse)).get(verse.verse)?.highlight ?? null);
	const status = $derived.by(() => {
		if (note !== saved) return 'Editing…';
		if (!saved) return '';
		if (savedAt === null) return 'Saved';
		const secs = clock - savedAt;
		if (secs < 60) return 'Saved · just now';
		if (secs < 3600) return `Saved · ${Math.floor(secs / 60)} min ago`;
		return `Saved · ${ago(savedAt)}`;
	});

	/** The note marker, or where it would be when the verse isn't on screen. */
	function marker(): DOMRect {
		return app.reader?.markerRect(verse) ?? new DOMRect(innerWidth / 2 + 190, 190, 20, 20);
	}

	function anchor(): DOMRect {
		const m = marker();
		const x = m.left + m.width / 2;
		const left = Math.max(8, Math.min(x - TIP, innerWidth - WIDTH - 8));
		tip = Math.max(left + 24, Math.min(x, left + WIDTH - 24)) - left;
		return new DOMRect(x - TIP, m.top, 0, m.height);
	}

	function save() {
		clearTimeout(timer);
		if (deleted || note === saved) return;
		app.dispatch({ type: 'saveNote', verse, body: note });
		saved = note;
		savedAt = now();
		clock = savedAt;
	}

	function close() {
		save();
		app.note = null;
	}

	function edited() {
		clearTimeout(timer);
		timer = setTimeout(save, AUTOSAVE_MS);
	}

	function keydown(e: KeyboardEvent) {
		const command = isMac ? e.metaKey : e.ctrlKey;
		if (e.key === 'Enter' && command && !e.shiftKey && !e.altKey) {
			e.preventDefault();
			close();
		}
	}

	onMount(() => {
		editor?.setSelectionRange(note.length, note.length);
		const tick = setInterval(() => (clock = now()), 30_000);
		return () => {
			clearInterval(tick);
			save();
		};
	});
</script>

<Popover
	{anchor}
	gap={5 + BEAK}
	flip={false}
	onclose={close}
	label="Note on {reference}"
	radius={16}
	style="width: {WIDTH}px"
	class="note-popover"
>
	<svg class="beak" style:left="{tip - 9}px" width="18" height="10" viewBox="0 0 18 10" aria-hidden="true">
		<path d="M0 9.5 9 0.5 18 9.5" />
	</svg>
	<div class="body">
		<header class="head">
			<span class="glyph"><Icon name="note" size={16} heavy /></span>
			<span class="reference">{reference}</span>
			<span class="abbrev">{info?.abbrev ?? ''}</span>
			<span class="swatches">
				{#each HIGHLIGHT_COLORS as c (c)}
					<button
						type="button"
						class={['swatch', { on: colour === c }]}
						style:--swatch="var(--swatch-{c})"
						title={app.highlightName(c)}
						aria-label="{app.highlightName(c)} highlight"
						aria-pressed={colour === c}
						onclick={() => app.dispatch({ type: 'highlight', color: colour === c ? null : c })}
					>
						{#if colour === c}<Icon name="check" size={12} />{/if}
					</button>
				{/each}
			</span>
		</header>
		<blockquote class="quote" style:--rule={colour ? `var(--swatch-${colour})` : 'var(--accent)'}>{quote}</blockquote>
		<div class="editor">
			<textarea
				bind:this={editor}
				bind:value={note}
				oninput={edited}
				onkeydown={keydown}
				placeholder="Write a note…"
				aria-label="Note"
				spellcheck="true"
			></textarea>
		</div>
		<footer class="foot">
			<button
				type="button"
				class="delete"
				title="Delete note"
				aria-label="Delete note"
				onclick={() => {
					deleted = true;
					clearTimeout(timer);
					app.dispatch({ type: 'removeNote', verse });
					app.note = null;
				}}
			>
				<Icon name="trash" size={16} heavy />
			</button>
			<span class="status" aria-live="polite">{status}</span>
			<button type="button" class="done" title="Done ({isMac ? '⌘↩' : 'Ctrl+Enter'})" onclick={close}>Done</button>
		</footer>
	</div>
</Popover>

<style>
	.beak {
		position: absolute;
		top: -9px;
		overflow: visible;
	}

	.beak path {
		fill: var(--surface);
		stroke: var(--border);
		stroke-width: 0.75;
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 16px;
	}

	.head {
		display: flex;
		align-items: center;
		height: 16px;
	}

	.glyph {
		display: flex;
		color: var(--accent);
	}

	.reference {
		margin-left: 6px;
		color: var(--ink);
		font: var(--type-ui-strong);
		white-space: nowrap;
	}

	.abbrev {
		margin-left: 6px;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.swatches {
		display: flex;
		gap: 6px;
		margin-left: auto;
	}

	.swatch {
		display: grid;
		place-items: center;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--swatch);
		box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.18);
		color: #1c1d21;
	}

	.swatch:hover:not(.on) {
		box-shadow:
			inset 0 0 0 0.5px rgb(0 0 0 / 0.18),
			0 0 0 3px var(--fill-strong);
	}

	.swatch.on {
		box-shadow:
			inset 0 0 0 0.5px rgb(0 0 0 / 0.18),
			0 0 0 2px var(--surface),
			0 0 0 3.5px var(--ink);
	}

	.quote {
		display: -webkit-box;
		margin: 0;
		padding-left: 12px;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		background: linear-gradient(var(--rule), var(--rule)) left / 2px 100% no-repeat;
		color: var(--ink-2);
		font: italic 400 14px/20px var(--font-reading);
	}

	.editor {
		height: 100px;
		padding: 10px 12px;
		border-radius: 10px;
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
	}

	.editor:focus-within {
		box-shadow:
			inset 0 0 0 1px var(--accent),
			0 0 0 3px var(--focus);
	}

	textarea {
		display: block;
		width: 100%;
		height: 100%;
		padding: 0;
		border: 0;
		resize: none;
		background: none;
		color: var(--ink);
		caret-color: var(--accent);
		font: 400 14px/20px var(--font-ui);
		outline: none;
	}

	textarea::placeholder {
		color: var(--ink-3);
	}

	.foot {
		display: flex;
		align-items: center;
		height: 28px;
	}

	.delete {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		color: var(--danger);
	}

	.delete:hover {
		background: var(--fill);
	}

	.delete:active {
		background: var(--fill-strong);
	}

	.status {
		flex: 1;
		margin-left: 8px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.done {
		height: 24px;
		padding: 0 10px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		font: 600 12px/16px var(--font-ui);
	}

	.done:hover {
		background-image: linear-gradient(rgb(0 0 0 / 0.07), rgb(0 0 0 / 0.07));
	}

	.done:active {
		background-image: linear-gradient(rgb(0 0 0 / 0.14), rgb(0 0 0 / 0.14));
	}
</style>
