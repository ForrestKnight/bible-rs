<!--
	⌘K (palette.rs): jump to a reference ("jn 3:16", "Rom 8", "1co13"), return to a recent place,
	search, switch translation or run a command, all from the keyboard.
-->
<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { getApp, now, type Action } from '../app.svelte';
	import { suggestReferences, type Reference } from '../core';
	import { SHORTCUTS, isMac, keycaps, rowCaps } from '../keyboard';
	import { ago } from '../time';
	import { Icon, Keycap, type IconName } from '../ui';

	type Section = 'Go to' | 'Recent' | 'Translations' | 'Commands';

	interface Item {
		section: Section;
		icon: IconName;
		title: string;
		/** Characters of the title that match the query (set in the accent). */
		matched: boolean[];
		/** A second line: a verse (serif) or a detail. */
		subtitle?: { text: string; serif: boolean };
		trailing?: string;
		keys: string[];
		action: Action;
		/** What ⌘↵ does (open in parallel). */
		alternate?: Action;
	}

	const app = getApp();
	const id = $props.id();

	let query = $state(untrack(() => app.demo.palette ?? ''));
	let selected = $state(0);
	let input = $state<HTMLInputElement>();
	let results = $state<HTMLDivElement>();
	/** Recent reading, as it was when the palette opened. */
	const recent = untrack(() => app.readingSince(now() - 30 * 86_400));

	const bound = (shortcut: string) => {
		const s = SHORTCUTS.find((s) => s.id === shortcut);
		return s ? rowCaps(s, app.settings.shortcuts) : [];
	};
	const cmd = (key: string, shift = false) => keycaps({ ctrl: false, alt: false, shift, cmd: true, key });

	/** Which characters of `title` match `query`: its letters in order through the book name (as
	 * "jn" in John), and the numbers after it when the query has any. */
	function matches(title: string, q: string): boolean[] {
		const letters = [...q].filter((c) => /\p{L}/u.test(c)).map((c) => c.toLowerCase());
		const numbers = /\d/.test(q);
		const chars = [...title];
		let end = chars.length;
		for (let i = 1; i < chars.length; i++) {
			if (/\d/.test(chars[i]) && chars[i - 1] === ' ') {
				end = i - 1;
				break;
			}
		}
		const out = chars.map(() => false);
		let next = 0;
		for (let i = 0; i < end && next < letters.length; i++) {
			if (chars[i].toLowerCase() === letters[next]) {
				out[i] = true;
				next++;
			}
		}
		if (numbers) for (let i = end; i < chars.length; i++) out[i] = true;
		return out;
	}

	const command = (icon: IconName, title: string, keys: string[], action: Action): Item => ({
		section: 'Commands',
		icon,
		title,
		matched: [],
		keys,
		action
	});

	const items = $derived.by((): Item[] => {
		const core = app.core;
		if (!core) return [];
		const q = query.trim();
		const s = app.settings;
		const id = app.textTranslation();
		const abbrev = app.info(id)?.abbrev ?? '';
		const loaded = app.loaded.has(id);
		const name = (book: number) => core.books[book]?.name ?? '';
		const verseText = (r: Reference) => {
			const v = r.verse && loaded ? core.verse(id, { book: r.book, chapter: r.chapter, verse: r.verse }) : null;
			return v ? v.runs.map((run) => run.text).join('') : null;
		};
		const chapterDetail = (index: number) => {
			const verses = app.verses(id, index);
			if (!verses?.length) return undefined;
			const n = Math.max(...verses.map((v) => Math.max(v.number, v.numberEnd)));
			return { text: `Chapter · ${n} verses`, serif: false };
		};
		const out: Item[] = [];
		let first: Reference | undefined;

		if (q) {
			// Only references that exist in the translation being read (Jonah has no 3:16).
			const refs = suggestReferences(q, 8).filter(
				(r) => !loaded || (r.verse ? verseText(r) !== null : !!app.verses(id, r.chapterIndex)?.length)
			);
			first = refs[0];
			for (const r of refs.slice(0, 3)) {
				const title = r.verse
					? r.verseEnd && r.verseEnd > r.verse
						? `${name(r.book)} ${r.chapter}:${r.verse}–${r.verseEnd}`
						: `${name(r.book)} ${r.chapter}:${r.verse}`
					: `${name(r.book)} ${r.chapter}`;
				const text = verseText(r);
				const jump: Action = { type: 'jump', chapter: r.chapterIndex, verse: r.verse ?? undefined, flash: !!r.verse };
				out.push({
					section: 'Go to',
					icon: r.verse ? 'book' : 'list',
					title,
					matched: matches(title, q),
					subtitle: r.verse ? (text !== null ? { text, serif: true } : undefined) : chapterDetail(r.chapterIndex),
					trailing: abbrev,
					keys: [],
					action: jump,
					alternate: { type: 'openInParallel', then: jump }
				});
			}
			// The first reference's chapter, when it named a verse.
			if (first?.verse) {
				const title = `${name(first.book)} ${first.chapter}`;
				const jump: Action = { type: 'jump', chapter: first.chapterIndex, flash: false };
				out.push({
					section: 'Go to',
					icon: 'list',
					title,
					matched: matches(title, q),
					subtitle: chapterDetail(first.chapterIndex),
					trailing: abbrev,
					keys: [],
					action: jump,
					alternate: { type: 'openInParallel', then: jump }
				});
			}
		}

		// Recent reading: where each recent chapter was left.
		const seen: number[] = [];
		const limit = q ? 2 : 6;
		for (const r of recent) {
			const chapter = (core.books[r.book]?.firstChapter ?? 0) + r.chapter - 1;
			if (seen.includes(chapter)) continue;
			seen.push(chapter);
			if (seen.length > limit) break;
			const finished = r.verses > 0 && r.furthest >= r.verses;
			const started = r.furthest <= 1 && !finished;
			const stopped = !finished && r.furthest > 1;
			const detail = started ? 'started' : stopped ? `stopped at verse ${r.furthest}` : 'finished';
			const jump: Action = { type: 'jump', chapter, verse: stopped ? r.furthest : undefined, flash: false };
			out.push({
				section: 'Recent',
				icon: 'history',
				title: stopped ? `${name(r.book)} ${r.chapter}:${r.furthest}` : `${name(r.book)} ${r.chapter}`,
				matched: [],
				subtitle: { text: `${app.info(r.translation)?.abbrev ?? r.translation} · ${detail}`, serif: false },
				trailing: ago(r.started),
				keys: [],
				action: jump,
				alternate: { type: 'openInParallel', then: jump }
			});
		}

		if (q) {
			const ql = q.toLowerCase();
			for (const t of app.translations) {
				if (t.id === id || !(t.abbrev.toLowerCase().includes(ql) || t.name.toLowerCase().includes(ql))) continue;
				out.push({
					section: 'Translations',
					icon: 'book',
					title: `Read ${t.abbrev}`,
					matched: [],
					subtitle: { text: t.name, serif: false },
					keys: [],
					action: { type: 'setTranslation', id: t.id },
					alternate: { type: 'addParallel', id: t.id }
				});
			}
			out.push(command('search', `Search for “${q}”`, bound('search'), { type: 'openSearch', query: q }));
			if (first) {
				if (first.verse) {
					const verse = { book: first.book, chapter: first.chapter, verse: first.verse };
					out.push(
						command('compare', `Compare ${name(first.book)} ${first.chapter}:${first.verse} across translations`, bound('compare'), {
							type: 'compareVerse',
							verse
						})
					);
				}
				const jump: Action = {
					type: 'jump',
					chapter: first.chapterIndex,
					verse: first.verse ?? undefined,
					flash: !!first.verse
				};
				out.push(
					command('columns', `Open ${name(first.book)} ${first.chapter} in parallel`, bound('parallel'), {
						type: 'openInParallel',
						then: jump
					})
				);
			}
		}

		const ql = q.toLowerCase();
		const commands: [string, IconName, string[], Action][] = [
			['Toggle parallel translations', 'columns', bound('parallel'), { type: 'toggleParallel' }],
			['Theme: Light', 'text-size', [], { type: 'setTheme', theme: 'light' }],
			['Theme: Sepia', 'text-size', [], { type: 'setTheme', theme: 'sepia' }],
			['Theme: Dark', 'text-size', [], { type: 'setTheme', theme: 'dark' }],
			[
				s.typo.mode === 'paragraph' ? 'Layout: verse per line' : 'Layout: paragraphs',
				'list',
				bound('mode'),
				{ type: 'toggleMode' }
			],
			[s.typo.redLetters ? 'Red letters: off' : 'Red letters: on', 'text-size', bound('red'), { type: 'toggleRedLetters' }],
			['Bigger text', 'text-size', cmd('+'), { type: 'fontSize', delta: 1 }],
			['Smaller text', 'text-size', cmd('-'), { type: 'fontSize', delta: -1 }],
			['Toggle sidebar', 'sidebar', bound('sidebar'), { type: 'toggleSidebar' }],
			['Export notes as Markdown…', 'note', cmd('E'), { type: 'export', kind: 'markdown' }],
			['Export notes as JSON…', 'note', cmd('E', true), { type: 'export', kind: 'json' }],
			['Import a translation…', 'book', cmd('I'), { type: 'importTranslation' }],
			['Manage translations & API keys', 'book', [], { type: 'openSettings', tab: 'translations' }],
			['Settings', 'sidebar', cmd(','), { type: 'openSettings', tab: 'reading' }],
			['Keyboard shortcuts', 'list', cmd('/'), { type: 'showShortcuts' }]
		];
		for (const [title, icon, keys, action] of commands) {
			if (!q || title.toLowerCase().includes(ql)) out.push(command(icon, title, keys, action));
		}
		return out;
	});

	const current = $derived(Math.max(0, Math.min(selected, items.length - 1)));

	/** The title in runs, matched and not. */
	function runs(it: Item): { text: string; hit: boolean }[] {
		if (!it.matched.some(Boolean)) return [{ text: it.title, hit: false }];
		const out: { text: string; hit: boolean }[] = [];
		[...it.title].forEach((c, i) => {
			const hit = it.matched[i] ?? false;
			const last = out.at(-1);
			if (last && last.hit === hit) last.text += c;
			else out.push({ text: c, hit });
		});
		return out;
	}

	function close() {
		app.palette = false;
	}

	function run(a: Action) {
		close();
		app.dispatch(a);
	}

	async function move(delta: number) {
		if (!items.length) return;
		selected = (current + delta + items.length) % items.length;
		await tick();
		const row = results?.querySelector<HTMLElement>(`#${CSS.escape(`${id}-${selected}`)}`);
		if (!results || !row) return;
		const y = row.getBoundingClientRect().top - results.getBoundingClientRect().top + results.scrollTop;
		const h = row.offsetHeight;
		const view = results.clientHeight;
		if (y < results.scrollTop) results.scrollTop = y - 28;
		else if (y + h > results.scrollTop + view) results.scrollTop = y + h + 8 - view;
	}

	function onkeydown(e: KeyboardEvent) {
		const plain = !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey;
		const command = isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
		if (e.key === 'ArrowDown' && plain) void move(1);
		else if (e.key === 'ArrowUp' && plain) void move(-1);
		else if (e.key === 'Escape' && plain) close();
		else if (e.key === 'Tab') {
			/* The field is the palette's only stop. */
		}
		else if (e.key === 'Enter' && !e.isComposing && (plain || (command && !e.altKey && !e.shiftKey))) {
			const it = items[current];
			if (it) run(plain ? it.action : (it.alternate ?? it.action));
			else close();
		} else return;
		e.preventDefault();
		e.stopPropagation();
	}

	onMount(() => {
		const before = document.activeElement as HTMLElement | null;
		input?.focus();
		input?.setSelectionRange(query.length, query.length);
		return () => {
			const now = document.activeElement;
			if (before?.isConnected && (!now || now === document.body)) before.focus({ preventScroll: true });
		};
	});
</script>

<div class="backdrop" role="presentation" onpointerdown={close}></div>
<div class="palette" role="dialog" aria-modal="true" aria-label="Go to, search or run a command">
	<div class="query">
		<span class="glass"><Icon name="search" size={20} /></span>
		<input
			bind:this={input}
			bind:value={query}
			oninput={() => (selected = 0)}
			{onkeydown}
			type="text"
			role="combobox"
			aria-expanded="true"
			aria-controls="{id}-list"
			aria-activedescendant={items.length ? `${id}-${current}` : undefined}
			aria-autocomplete="list"
			placeholder="Go to a reference, search, or run a command"
			autocomplete="off"
			autocorrect="off"
			autocapitalize="off"
			spellcheck="false"
		/>
		<Keycap label="esc" />
	</div>
	<div class="results" bind:this={results} id="{id}-list" role="listbox" aria-label="Results">
		{#each items as it, i (i)}
			{#if i === 0 || items[i - 1].section !== it.section}
				<div class="section" role="presentation">{it.section}</div>
			{/if}
			<div
				class={['row', { selected: i === current, tall: !!it.subtitle }]}
				id="{id}-{i}"
				role="option"
				tabindex="-1"
				aria-selected={i === current}
				onclick={() => run(it.action)}
				onkeydown={(e) => e.key === 'Enter' && run(it.action)}
			>
				<span class="tile"><Icon name={it.icon} size={16} heavy /></span>
				<span class="text">
					<span class={['title', { fuzzy: it.matched.some(Boolean) }]}
						>{#each runs(it) as r, j (j)}{#if r.hit}<span class="hit">{r.text}</span>{:else}{r.text}{/if}{/each}</span
					>
					{#if it.subtitle}
						<span class={['subtitle', { serif: it.subtitle.serif }]}>{it.subtitle.text.replace(/\n/g, ' ')}</span>
					{/if}
				</span>
				{#if it.trailing}<span class="trailing">{it.trailing}</span>{/if}
				{#if it.keys.length}
					<span class="keys">{#each it.keys as k, j (j)}<Keycap label={k} />{/each}</span>
				{:else if i === current}
					<span class="keys enter"><Keycap label="↵" /></span>
				{/if}
			</div>
		{/each}
	</div>
	<div class="footer">
		<span class="hint"><Keycap label="↑↓" />Move</span>
		<span class="hint"><Keycap label="↵" />Open</span>
		<span class="hint"><Keycap label={isMac ? '⌘↵' : 'Ctrl+↵'} />Open in parallel</span>
		<span class="tip">Type a reference, book or command</span>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: var(--scrim);
		z-index: 70;
	}

	.palette {
		position: fixed;
		top: 96px;
		left: round(calc(50% - min(600px, 100vw - 40px) / 2), 1px);
		width: min(600px, calc(100vw - 40px));
		display: flex;
		flex-direction: column;
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-modal);
		overflow: hidden;
		z-index: 71;
	}

	.query {
		display: flex;
		align-items: center;
		height: 56px;
		padding: 0 16px;
		gap: 12px;
		box-shadow: 0 1px 0 var(--separator);
		margin-bottom: 1px;
	}

	.glass {
		display: flex;
		color: var(--ink-2);
	}

	input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: 0;
		padding: 0;
		background: none;
		color: var(--ink);
		caret-color: var(--accent);
		font: 400 17px/24px var(--font-ui);
		outline: none;
	}

	input::placeholder {
		color: var(--ink-3);
		font-size: 13px;
	}

	.results {
		max-height: max(120px, calc(100vh - 96px - 56px - 1px - 40px - 48px));
		overflow-y: auto;
		padding: 4px 0 6px;
		overscroll-behavior: contain;
	}

	.section {
		height: 24px;
		margin-bottom: 2px;
		padding: 6px 18px 0;
		color: var(--ink-3);
		font: var(--type-caption);
	}

	.row {
		display: flex;
		align-items: center;
		height: 40px;
		margin: 0 8px 2px;
		padding: 0 12px;
		border-radius: 10px;
		cursor: default;
		outline: none;
	}

	.row.tall {
		height: 48px;
	}

	.row:hover {
		background: var(--fill);
	}

	.row.selected {
		background: var(--accent-soft);
	}

	.tile {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		width: 28px;
		height: 28px;
		margin-right: 12px;
		border-radius: 8px;
		background: var(--fill);
		color: var(--ink-2);
	}

	.selected .tile {
		background: var(--raised);
		color: var(--accent);
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-right: 8px;
	}

	.title,
	.subtitle {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.title {
		color: var(--ink);
		font: 600 13px/16px var(--font-ui);
	}

	.title.fuzzy {
		font-weight: 500;
	}

	.hit {
		color: var(--accent);
		font-weight: 600;
	}

	.subtitle {
		color: var(--ink-2);
		font: 400 13px/16px var(--font-ui);
	}

	/* egui sets Literata's baseline lower in a 16 pt line than CSS does. */
	.subtitle.serif {
		position: relative;
		top: 2.5px;
		font-family: var(--font-reading);
	}

	.trailing {
		flex-shrink: 0;
		margin-left: 8px;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.keys {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
		margin-left: 4px;
	}

	.keys.enter {
		margin-left: 8px;
	}

	.footer {
		display: flex;
		align-items: center;
		flex-shrink: 0;
		height: 40px;
		padding: 0 16px;
		gap: 16px;
		background: var(--fill);
		box-shadow: inset 0 1px 0 var(--separator);
	}

	.hint {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--ink-3);
		font: var(--type-small);
		white-space: nowrap;
	}

	.tip {
		margin-left: auto;
		color: var(--ink-3);
		font: var(--type-small);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
