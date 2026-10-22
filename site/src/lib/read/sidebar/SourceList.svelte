<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { getApp, type Page, type StudyKind } from '../app.svelte';
	import type { Book } from '../core';
	import { Icon, type IconName } from '../ui';
	import ChapterGrid from './ChapterGrid.svelte';
	import { activity as marksActivity, summary } from './marks';
	import Row from './Row.svelte';
	import StudyItems from './StudyItems.svelte';
	import { STUDY_KINDS, type SourceListState } from './state.svelte';

	let { st }: { st: SourceListState } = $props();

	const app = getApp();

	const STUDY: { kind: StudyKind; label: string; icon: IconName }[] = [
		{ kind: 'bookmarks', label: 'Bookmarks', icon: 'bookmark' },
		{ kind: 'highlights', label: 'Highlights', icon: 'highlighter' },
		{ kind: 'notes', label: 'Notes', icon: 'note' }
	];

	let list = $state<HTMLElement>();
	let field = $state<HTMLInputElement>();
	let currentRow = $state<HTMLElement>();

	const books = $derived(app.books);
	const reading = $derived(app.page === 'reader');
	const currentBook = $derived(books.findLast((b) => b.firstChapter <= app.here.chapter)?.index ?? -1);
	const counts = $derived(app.counts());
	const activePlans = $derived(app.plans().filter((p) => p.finished === null).length);
	const abbrev = $derived(app.info(app.textTranslation())?.abbrev ?? '');
	const filter = $derived(st.filter.trim().toLowerCase());
	const testaments = $derived(
		(['Old Testament', 'New Testament'] as const).map((label, t) => ({
			label,
			books: books.filter((b) => +b.newTestament === t && (!filter || b.name.toLowerCase().includes(filter)))
		}))
	);

	/** Highlights and notes by chapter and by book (rings on chapter chips and book rows). */
	const activity = $derived(marksActivity(app.studyItems('highlights'), app.studyItems('notes')));

	/** Where the reader is, then where it came from: three chapters. */
	const recent = $derived.by(() => {
		const out: { chapter: number; verse: number }[] = [];
		for (const loc of [app.here, ...app.history.back.toReversed()]) {
			if (out.length === 3) break;
			if (!out.some((l) => l.chapter === loc.chapter)) out.push({ chapter: loc.chapter, verse: loc.verse });
		}
		return out;
	});

	// The book being read: its testament opens, its chapters show and it scrolls into view. With
	// no reader to follow, the testaments it opened close, and reopen on return.
	$effect(() => {
		const book = currentBook;
		const r = reading;
		untrack(() => {
			if (!r) {
				for (const t of [0, 1]) {
					if (st.autoOpened[t]) {
						st.testaments[t] = false;
						st.autoOpened[t] = false;
					}
				}
				st.revealed = null;
				return;
			}
			if (book < 0 || st.revealed === book) return;
			st.revealed = book;
			const t = +books[book].newTestament;
			if (!st.testaments[t]) {
				st.testaments[t] = true;
				st.autoOpened[t] = true;
			}
			if (st.autoOpened[1 - t]) {
				st.testaments[1 - t] = false;
				st.autoOpened[1 - t] = false;
			}
			st.expanded = book;
			if (!st.filtering) void tick().then(reveal);
		});
	});

	/** Scrolls just enough to show the current book's row. */
	function reveal() {
		if (!list || !currentRow) return;
		const l = list.getBoundingClientRect();
		const r = currentRow.getBoundingClientRect();
		if (r.top < l.top) list.scrollTop += r.top - l.top;
		else if (r.bottom > l.bottom) list.scrollTop += r.bottom - l.bottom;
	}

	$effect(() => {
		if (!st.filterFocus) return;
		void tick().then(() => {
			if (!field) return;
			field.focus();
			field.setSelectionRange(field.value.length, field.value.length);
			if (list) list.scrollTop = 0;
		});
	});

	// Development renders: an expanded study collection (`study:bookmarks`), or filtering. The Mac
	// opens the collection before its first frame, so the book is revealed below it.
	$effect(() => {
		const d = app.demo.study;
		if (!d) return;
		untrack(() => {
			if (d === 'filter') st.startFilter('');
			else if (STUDY_KINDS.includes(d as StudyKind)) {
				const i = STUDY_KINDS.indexOf(d as StudyKind);
				if (!st.studyOpen[i]) st.toggleStudy(d as StudyKind);
				void tick().then(reveal);
			}
		});
	});

	function openPage(page: Page) {
		app.dispatch({ type: 'openPage', page });
	}

	function pickBook(b: Book) {
		if (b.chapters === 1) app.dispatch({ type: 'jump', chapter: b.firstChapter });
		else st.expanded = st.expanded === b.index ? null : b.index;
	}

	function placeLabel(i: number, chapter: number, verse: number) {
		const sel = app.selection?.[0];
		const name = app.chapterName(chapter);
		return i === 0 && sel && app.chapterIndex(sel) === chapter ? `${name}:${sel.verse}` : name;
	}
</script>

<div class="list" bind:this={list}>
	{#if st.filtering}
		<div class="filter">
			<Icon name="search" size={14} class="glass" />
			<input
				bind:this={field}
				bind:value={st.filter}
				type="text"
				placeholder="Filter books"
				aria-label="Filter books"
				spellcheck="false"
				autocomplete="off"
				onkeydown={(e) => {
					if (e.key === 'Escape') {
						e.preventDefault();
						e.stopPropagation();
						st.endFilter();
					}
				}}
				onblur={() => {
					if (document.hasFocus() && !st.filter.trim()) st.endFilter();
				}}
			/>
		</div>
		{#if recent.length}
			<h3 class="section first">Recent</h3>
			{#each recent as place, i (place.chapter)}
				<Row
					icon="clock"
					quietIcon
					label={placeLabel(i, place.chapter, place.verse)}
					trailing={abbrev}
					onclick={() =>
						app.dispatch({ type: 'jump', chapter: place.chapter, verse: place.verse, flash: place.verse > 1 })}
				/>
			{/each}
		{/if}
	{:else}
		<h3 class="section first">Study</h3>
		{#each STUDY as s, i (s.kind)}
			{@const n = counts[s.kind]}
			<!-- Rows open their page; the chevron (shown on hover, or while open) lists the most
			     recent items in place. -->
			<Row
				icon={s.icon}
				label={s.label}
				count={n > 0 ? n : null}
				disclosure={st.studyOpen[i]}
				disclosureOnHover
				selected={app.page === s.kind}
				onclick={() => openPage(s.kind)}
				onchevron={() => st.toggleStudy(s.kind)}
			/>
			{#if st.studyOpen[i]}
				<StudyItems
					kind={s.kind}
					all={st.studyAll[i]}
					onshowall={() => (st.studyAll[i] = true)}
					filter={st.highlightFilter}
					onfilter={(c) => (st.highlightFilter = c)}
				/>
			{/if}
		{/each}
		<Row icon="history" label="History" selected={app.page === 'history'} onclick={() => openPage('history')} />
		<Row
			icon="checklist"
			label="Reading Plans"
			count={activePlans > 0 ? activePlans : null}
			selected={app.page === 'plans'}
			onclick={() => openPage('plans')}
		/>
		<h3 class="section">Library</h3>
		<Row
			icon="globe"
			label="Translations"
			count={app.translations.length}
			selected={app.page === 'translations'}
			onclick={() => openPage('translations')}
		/>
	{/if}

	<h3 class="section">Books</h3>
	{#each testaments as group, t (group.label)}
		{#if group.books.length}
			{@const open = st.testaments[t] || filter !== ''}
			<Row
				icon="book"
				label={group.label}
				count={group.books.length}
				disclosure={open}
				onclick={() => {
					st.testaments[t] = !open;
					st.autoOpened[t] = false;
				}}
			/>
			{#if open}
				{#each group.books as b (b.index)}
					{@const here = b.index === currentBook && reading}
					{@const marked = activity.books.get(b.index)}
					{#if here}
						<Row
							bind:el={currentRow}
							label={b.name}
							indent={24}
							count={b.chapters}
							marks={marked?.marks}
							title={marked && summary(marked)}
							selected
							href={app.bookHref(b)}
							onclick={() => pickBook(b)}
						/>
					{:else}
						<Row
							label={b.name}
							indent={24}
							marks={marked?.marks}
							title={marked && summary(marked)}
							href={app.bookHref(b)}
							onclick={() => pickBook(b)}
						/>
					{/if}
					{#if st.expanded === b.index && b.chapters > 1}
						<ChapterGrid
							book={b}
							current={app.here.chapter}
							activity={activity.chapters}
							href={(chapter) => app.chapterHref(chapter)}
							onpick={(chapter) => app.dispatch({ type: 'jump', chapter })}
						/>
					{/if}
				{/each}
			{/if}
		{/if}
	{/each}
</div>

<style>
	.list {
		height: 100%;
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: 4px 8px 8px;
	}

	.section {
		display: flex;
		align-items: center;
		height: 36px;
		padding: 16px 8px 0;
		color: var(--ink-3);
		font: var(--type-caption);
		letter-spacing: var(--type-caption-tracking);
	}

	.section.first {
		height: 28px;
		padding-top: 8px;
	}

	.filter {
		position: relative;
		display: flex;
		align-items: center;
		height: 28px;
		margin-bottom: 4px;
		border-radius: 8px;
		background: var(--fill);
	}

	.filter:focus-within {
		background: var(--field);
		box-shadow:
			inset 0 0 0 1px var(--accent),
			0 0 0 3px var(--focus);
	}

	.filter :global(.glass) {
		position: absolute;
		left: 10px;
		color: var(--ink-3);
	}

	input {
		width: 100%;
		height: 100%;
		padding: 0 10px 0 30px;
		border: 0;
		background: none;
		outline: none;
		color: var(--ink);
		font: var(--type-ui);
	}

	input::placeholder {
		color: var(--ink-3);
	}
</style>
