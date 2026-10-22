<!--
	Notes (pages/notes.rs): the notes in a list beside an editor for the selected one — the verse
	quoted on its highlight, a format bar, the note as styled Markdown, and the verses it mentions
	as links. The Mac styles Markdown as it's typed; here the note shows styled, and becomes its
	Markdown to edit while the field has focus.
-->
<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { getApp, now } from '../app.svelte';
	import { parseReference, type VerseRef } from '../core';
	import ListItem from '../sidebar/ListItem.svelte';
	import { ago, shortDate } from '../time';
	import { Icon, Menu, MenuItem, Popover, RoundButton, Segmented, TextField } from '../ui';
	import { apply, lines, mentioned, notesPage, plain, sameVerse, stylesAt, type Tool } from './notes.svelte';

	let { top }: { top: number } = $props();

	const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

	const app = getApp();
	const notes = $derived(app.studyItems('notes'));
	const index = (v: VerseRef) => app.chapterIndex(v);
	const verseText = (v: VerseRef) =>
		app.core?.verse(app.textTranslation(), v)?.runs.map((r) => r.text).join('').trim() ?? '';
	const marks = (v: VerseRef) => app.marks(app.textTranslation(), index(v)).get(v.verse);

	// The demo's `note-select:<reference>` opens a note.
	$effect(() => {
		const r = parseReference(app.demo['note-select'] ?? '');
		if (r?.verse) notesPage.selected = { book: r.book, chapter: r.chapter, verse: r.verse };
	});

	/** The selected note, else the newest. */
	const selected = $derived(
		notesPage.selected ?? [...notes].sort((a, b) => b.updated - a.updated)[0]?.verse ?? null
	);
	const note = $derived(selected ? app.noteFor(selected) : null);
	const saved = $derived(note?.body ?? '');
	/** What's typed and not saved yet, on the verse it's typed on: another tab saving the note keeps it. */
	let typed = $state<{ verse: VerseRef; text: string } | null>(null);
	const text = $derived(typed && selected && sameVerse(typed.verse, selected) ? typed.text : saved);
	const setText = (body: string) => selected && (typed = { verse: selected, text: body });
	let timer: ReturnType<typeof setTimeout> | undefined;
	let editing = $state(false);
	let field = $state<HTMLTextAreaElement>();
	let caret = $state(0);

	function flush() {
		clearTimeout(timer);
		if (selected && text !== saved) app.dispatch({ type: 'saveNote', verse: selected, body: text });
		typed = null;
	}

	function changed() {
		clearTimeout(timer);
		timer = setTimeout(flush, 600);
	}

	function select(v: VerseRef) {
		flush();
		editing = false;
		notesPage.selected = v;
	}

	onDestroy(flush);

	const items = $derived.by(() => {
		const list = notes.map((n) => ({
			verse: n.verse,
			reference: n.reference,
			body: n.body,
			updated: n.updated,
			snippet: n.text,
			highlight: n.highlight
		}));
		if (selected && !list.some((n) => sameVerse(n.verse, selected)))
			list.push({
				verse: selected,
				reference: app.label(selected),
				body: text,
				updated: now(),
				snippet: verseText(selected),
				highlight: marks(selected)?.highlight ?? null
			});
		const canonical = (a: VerseRef, b: VerseRef) => index(a) - index(b) || a.verse - b.verse;
		list.sort((a, b) => (notesPage.recent ? b.updated - a.updated : 0) || canonical(a.verse, b.verse));
		const q = app.pageQuery.trim().toLowerCase();
		return list.filter(
			(n) =>
				!q ||
				n.reference.toLowerCase().includes(q) ||
				plain(n.body).toLowerCase().includes(q) ||
				n.snippet.toLowerCase().includes(q)
		);
	});

	// The new-note and link-verse fields.
	let adding = $state(false);
	let addButton = $state<HTMLButtonElement>();
	let addText = $state('');
	let linking = $state(false);
	let linkAnchor = $state<HTMLElement | null>(null);
	let linkText = $state('');
	let more = $state(false);
	let moreButton = $state<HTMLButtonElement>();

	function add(e: KeyboardEvent) {
		if (e.key !== 'Enter') return;
		const r = parseReference(addText);
		if (!r?.verse) return;
		select({ book: r.book, chapter: r.chapter, verse: r.verse });
		addText = '';
		adding = false;
	}

	function link(e: KeyboardEvent) {
		if (e.key !== 'Enter') return;
		const r = parseReference(linkText);
		if (!r?.verse) return;
		const label = app.label({ book: r.book, chapter: r.chapter, verse: r.verse });
		setText(`${text}${text && !text.endsWith('\n') ? '\n' : ''}- ${label}`);
		changed();
		linkText = '';
		linking = false;
	}

	function remove() {
		if (!selected) return;
		clearTimeout(timer);
		typed = null;
		app.dispatch({ type: 'removeNote', verse: selected });
		notesPage.selected = null;
		editing = false;
	}

	function copy(body: string, toast?: string) {
		void navigator.clipboard.writeText(body);
		if (toast) app.toastNow(toast);
	}

	// The editor: the note styled, or its Markdown while focused.
	async function edit() {
		editing = true;
		await tick();
		field?.focus();
		field?.setSelectionRange(text.length, text.length);
	}

	const track = () => (caret = field?.selectionStart ?? 0);
	const active = $derived(editing ? stylesAt(text, caret) : null);

	async function format(tool: Tool, button: HTMLElement) {
		if (tool === 'link') {
			linkAnchor = button;
			linking = true;
			return;
		}
		if (!editing) await edit();
		if (!field) return;
		const [next, a, b] = apply(tool, text, field.selectionStart, field.selectionEnd);
		setText(next);
		changed();
		await tick();
		field.setSelectionRange(a, b);
		track();
	}

	const words = $derived(text.split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length);
	const linked = $derived(mentioned(text));
	const meta = $derived.by(() => {
		const name = app.info(app.textTranslation())?.name ?? '';
		if (!note) return `${name} · New note`;
		const c = new Date(note.created * 1000);
		const edited = ago(note.updated);
		return `${name} · Created ${c.getDate()} ${MONTHS[c.getMonth()]} · Edited ${edited === 'Just now' ? 'just now' : edited}`;
	});

	const TOOLS: [Tool, string][] = [
		['heading', 'Heading'],
		['bold', 'Bold'],
		['italic', 'Italic'],
		['list', 'List'],
		['quote', 'Quote'],
		['link', 'Link a verse']
	];
</script>

<div class="notes" style:--toolbar="{top}px">
	<div class="list-pane">
		<div class="list-head">
			<h1 class="title">Notes</h1>
			<RoundButton icon="plus" label="New note" bind:el={addButton} onclick={() => (adding = !adding)} />
		</div>
		<div class="sub">
			<span>{notes.length === 1 ? '1 note' : `${notes.length} notes`}</span>
			<Segmented
				layout="tight"
				label="Order"
				options={[
					{ value: 0, label: 'Recent' },
					{ value: 1, label: 'Canonical' }
				]}
				value={notesPage.recent ? 0 : 1}
				onchange={(v) => (notesPage.recent = v === 0)}
			/>
		</div>
		<div class="list">
			{#each items as n (n.reference)}
				<ListItem
					reference={n.reference}
					trailing={shortDate(n.updated)}
					bar={n.highlight ? `var(--swatch-${n.highlight})` : undefined}
					note={plain(n.body) || 'New note'}
					snippet={n.snippet}
					selected={!!selected && sameVerse(n.verse, selected)}
					onclick={() => select(n.verse)}
				/>
			{/each}
		</div>
	</div>

	{#if selected}
		{@const v = selected}
		{@const m = marks(v)}
		<div class="editor-pane">
			<div class="editor">
				<div class="editor-head">
					<h2 class="title">{app.label(v)}</h2>
					<div class="actions">
						<RoundButton
							icon={m?.bookmark ? 'bookmark-fill' : 'bookmark'}
							label={m?.bookmark ? 'Remove bookmark' : 'Bookmark'}
							size={32}
							onclick={() => app.dispatch({ type: 'toggleBookmarkAt', verse: v })}
						/>
						<RoundButton
							icon="share"
							label="Copy note"
							size={32}
							onclick={() => copy(`${app.label(v)} — ${verseText(v)}\n\n${text}`, `Copied the note on ${app.label(v)}`)}
						/>
						<RoundButton icon="trash" label="Delete note" size={32} onclick={remove} />
						<RoundButton icon="ellipsis" label="More" size={32} bind:el={moreButton} onclick={() => (more = !more)} />
					</div>
				</div>
				<p class="meta">{meta}</p>

				<div class="quote" style:--bar={m?.highlight ? `var(--swatch-${m.highlight})` : 'var(--border)'}>
					<p class="quote-text" style:--wash={m?.highlight ? `var(--hl-${m.highlight})` : 'transparent'}>
						<span class="vn">{v.verse}</span><span class="wash">{verseText(v)}</span>
					</p>
				</div>

				<div class="tools">
					<div class="format" role="toolbar" aria-label="Format">
						{#each TOOLS as [tool, label], i (tool)}
							{#if i === 1}<span class="divider"></span>{/if}
							<button
								class={['tool', { on: active?.[tool as Exclude<Tool, 'link'>] }]}
								aria-label={label}
								title={label}
								onmousedown={(e) => e.preventDefault()}
								onclick={(e) => format(tool, e.currentTarget)}
							>
								{#if tool === 'heading'}<Icon name="text-size" size={16} heavy />
								{:else if tool === 'bold'}<b>B</b>
								{:else if tool === 'italic'}<i>I</i>
								{:else if tool === 'list'}<Icon name="list" size={16} heavy />
								{:else if tool === 'quote'}<Icon name="quote" size={16} heavy />
								{:else}<Icon name="link" size={16} heavy />{/if}
							</button>
						{/each}
					</div>
					<span class="count">Markdown · {words} {words === 1 ? 'word' : 'words'}</span>
				</div>

				{#if editing}
					<textarea
						class="body"
						bind:this={field}
						bind:value={() => text, setText}
						placeholder="Write a note…"
						aria-label="Note on {app.label(v)}"
						oninput={() => (changed(), track())}
						onselect={track}
						onkeyup={track}
						onclick={track}
						onblur={() => (flush(), (editing = false))}
					></textarea>
				{:else}
					<div class="body rendered" role="textbox" tabindex="0" aria-label="Note on {app.label(v)}" onfocus={edit}>
						{#if text.trim()}
							{#each lines(text) as line, i (i)}
								<div class={['line', line.kind]}>
									{#if line.kind === 'bullet'}<span class="dot">• </span>{/if}
									{#each line.runs as r, j (j)}
										{#if r.bold}<b>{r.text}</b>{:else if r.italic}<i>{r.text}</i>{:else}{r.text}{/if}
									{/each}
								</div>
							{/each}
						{:else}
							<span class="placeholder">Write a note…</span>
						{/if}
					</div>
				{/if}

				<div class="linked">
					<span class="linked-label">Linked</span>
					{#each linked as r (`${r.book}:${r.chapter}:${r.verse}`)}
						<button
							class="chip"
							onclick={() => app.dispatch({ type: 'jump', chapter: index(r), verse: r.verse, flash: true })}
						>
							<Icon name="link" size={12} heavy />{app.label(r)}
						</button>
					{/each}
					<button
						class="chip add"
						onclick={(e) => {
							linkAnchor = e.currentTarget;
							linking = !linking;
						}}
					>
						<Icon name="plus" size={12} heavy />Link verse
					</button>
				</div>
			</div>
		</div>

		{#if more}
			<Menu anchor={moreButton ?? null} align="end" width={180} onclose={() => (more = false)} label="More">
				<MenuItem
					label="Open in Reader"
					onclick={() => ((more = false), app.dispatch({ type: 'jump', chapter: index(v), verse: v.verse, flash: true }))}
				/>
				<MenuItem label="Copy as Markdown" onclick={() => ((more = false), copy(text))} />
			</Menu>
		{/if}
	{/if}
</div>

{#if adding}
	<Popover anchor={addButton ?? null} align="end" onclose={() => (adding = false)} label="New note">
		<div class="field-pop">
			<span>New note on…</span>
			<TextField bind:value={addText} placeholder="John 3:16" onkeydown={add} />
		</div>
	</Popover>
{/if}
{#if linking}
	<Popover anchor={linkAnchor} onclose={() => (linking = false)} label="Link a verse">
		<div class="field-pop">
			<span>Link a verse</span>
			<TextField bind:value={linkText} placeholder="Isaiah 9:2" onkeydown={link} />
		</div>
	</Popover>
{/if}

<style>
	.notes {
		position: absolute;
		inset: 0;
		display: flex;
		background: var(--paper);
		color: var(--ink);
		font: var(--type-ui);
	}

	.list-pane {
		position: relative;
		display: flex;
		flex: none;
		flex-direction: column;
		width: 336px;
		padding: calc(var(--toolbar) + 20px) 16px 0 32px;
	}

	.list-pane::after {
		position: absolute;
		top: var(--toolbar);
		right: 0;
		bottom: 0;
		width: 1px;
		background: var(--separator);
		content: '';
	}

	.list-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
	}

	.list-head :global(button) {
		margin-top: 4px;
	}

	.title {
		color: var(--ink);
		font: var(--type-heading);
		font-variation-settings: 'opsz' 28;
		letter-spacing: var(--type-heading-tracking);
	}

	.sub {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 32px;
		margin-top: 4px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.list {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 4px;
		margin: 8px -8px 0;
		padding: 4px 8px 24px;
		overflow-y: auto;
	}

	.editor-pane {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
	}

	.editor {
		padding: calc(var(--toolbar) + 20px) 40px 40px;
	}

	.editor-head {
		display: flex;
		justify-content: space-between;
	}

	.actions {
		display: flex;
		gap: 4px;
		width: 140px;
	}

	.meta {
		margin-top: 2px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.quote {
		position: relative;
		display: flow-root;
		box-sizing: border-box;
		max-width: 600px;
		margin-top: 20px;
		padding-left: 19px;
	}

	.quote::before {
		position: absolute;
		top: 0;
		bottom: 1px;
		left: 0;
		width: 3px;
		border-radius: 2px;
		background: var(--bar);
		content: '';
	}

	/* Each line on its own wash, 32 tall in a 34 pt row, as the Mac sets them. */
	.quote-text {
		display: -webkit-box;
		margin-top: -1px;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 6;
		line-clamp: 6;
		overflow: hidden;
		color: var(--ink);
		font: 400 20px/34px var(--font-reading);
	}

	.vn {
		position: relative;
		top: 4px;
		display: inline-block;
		margin-right: 4px;
		color: var(--verse);
		font: 600 11px/16px var(--font-ui);
		vertical-align: top;
	}

	.wash {
		padding: 1.15px 2px;
		border-radius: 4px;
		background: var(--wash);
		-webkit-box-decoration-break: clone;
		box-decoration-break: clone;
	}

	.tools {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 19px;
	}

	.format {
		display: flex;
		align-items: center;
		height: 32px;
		border-radius: 16px;
		background: var(--glass);
		box-shadow: var(--shadow-control);
	}

	.divider {
		width: 1px;
		height: 16px;
		background: var(--separator);
	}

	.tool {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		color: var(--ink);
	}

	.tool:hover {
		background: var(--fill);
	}

	.tool.on {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.tool b {
		font: 700 13px/16px var(--font-ui);
	}

	.tool i {
		font: italic 400 14px/16px var(--font-reading);
	}

	.count {
		color: var(--ink-3);
		font: var(--type-small);
	}

	/* The Mac's field is at least four 13 pt rows tall, and sets each 24 pt line's text 2.5 pt
	   above where CSS centres it. */
	.body {
		display: block;
		box-sizing: border-box;
		width: 100%;
		max-width: 560px;
		min-height: 65.5px;
		margin-top: 17.5px;
		padding: 0 0 2.5px;
		border: 0;
		outline: none;
		background: none;
		color: var(--ink);
		font: 400 15px/24px var(--font-ui);
		cursor: text;
	}

	textarea.body {
		resize: none;
		caret-color: var(--accent);
		field-sizing: content;
	}

	.line.gap {
		height: 8px;
	}

	.line.heading {
		font: 600 17px/24px var(--font-ui);
	}

	.line.quote {
		color: var(--ink-2);
		font-family: var(--font-reading);
		font-style: italic;
	}

	.line i {
		font-family: var(--font-reading);
	}

	.dot,
	.placeholder {
		color: var(--ink-3);
	}

	.linked {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin-top: 20px;
	}

	.linked-label {
		color: var(--ink-3);
		font: 600 12px/16px var(--font-ui);
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		height: 24px;
		padding: 0 10px;
		border-radius: 12px;
		background: var(--accent-soft);
		color: var(--accent);
		font: 600 12px/16px var(--font-ui);
	}

	.chip:hover {
		background: var(--selection);
	}

	.chip.add {
		background: none;
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--ink-2);
		font-weight: 500;
	}

	.chip.add:hover {
		background: var(--fill);
	}

	.field-pop {
		display: flex;
		flex-direction: column;
		gap: 8px;
		width: 220px;
		padding: 12px;
		color: var(--ink-2);
	}
</style>
