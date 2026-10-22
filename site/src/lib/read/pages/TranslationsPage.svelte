<!--
	Translations (pages/translations.rs): the library as a list with filter tabs and a sort order,
	beside an inspector for the one selected — its sample verse, facts and actions. The Mac's online
	translations and API keys aren't offered on the web; the NLT, read live, is in the translation
	menu only (../nlt.ts, DECISIONS.md).
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import type { TranslationInfo } from '../core';
	import { Badge, Button, CapsText, Icon, Menu, MenuItem, MenuSeparator, PillTab } from '../ui';
	import PageHead from './PageHead.svelte';

	const app = getApp();

	type Filter = 'all' | 'bundled' | 'imported';
	let filter = $state<Filter>('all');
	let sort = $state<'kind' | 'name'>('kind');
	let sorting = $state(false);
	let sortButton = $state<HTMLButtonElement>();
	let more = $state<{ id: string; el: HTMLElement } | null>(null);

	const LANGUAGES: Record<string, string> = {
		en: 'English',
		de: 'German',
		fr: 'French',
		es: 'Spanish',
		pt: 'Portuguese',
		it: 'Italian',
		nl: 'Dutch',
		la: 'Latin',
		el: 'Greek',
		grc: 'Greek',
		he: 'Hebrew',
		hbo: 'Hebrew'
	};

	const all = $derived(app.translations);
	const nb = $derived(all.filter((t) => !t.imported).length);
	const ni = $derived(all.length - nb);
	const subtitle = $derived(
		`${all.length} installed · ${[nb && `${nb} bundled`, ni && `${ni} imported`].filter(Boolean).join(', ')}`
	);

	const megabytes = (bytes: number) => `${(bytes / 1_048_576).toFixed(1)} MB`;
	const size = (t: TranslationInfo) => {
		const bytes = t.imported?.bytes ?? t.size;
		return bytes ? megabytes(bytes) : '';
	};
	const description = (t: TranslationInfo) =>
		t.imported ? t.imported.format : (t.description ?? t.license);
	const features = (t: TranslationInfo) =>
		[t.headings && 'Section headings', t.redLetters && 'Red letters'].filter(Boolean).join(', ') || '—';

	const shown = $derived.by(() => {
		const q = app.pageQuery.trim().toLowerCase();
		const out = all
			.filter((t) => filter === 'all' || (filter === 'imported') === !!t.imported)
			.filter(
				(t) =>
					!q ||
					t.abbrev.toLowerCase().includes(q) ||
					t.name.toLowerCase().includes(q) ||
					description(t).toLowerCase().includes(q)
			);
		if (sort === 'name') out.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
		return out;
	});

	const selectedId = $derived(app.translationPick ?? app.textTranslation());
	const selected = $derived(all.find((t) => t.id === selectedId));

	$effect(() => {
		if (selected) void app.ensure(selected.id);
	});

	const ready = $derived(!!selected && app.loaded.has(selected.id));
	const sample = $derived.by(() => {
		if (!ready || !selected) return '';
		const v = app.core?.verse(selected.id, { book: 18, chapter: 23, verse: 1 });
		return v?.runs.map((r) => r.text).join('').trim() ?? '';
	});
	const books = $derived.by(() => {
		const core = app.core;
		if (!ready || !selected || !core) return null;
		return core.books.filter((b) => core.verses(selected.id, b.firstChapter).length > 0).length;
	});

	function read(id: string) {
		app.dispatch({ type: 'setTranslation', id });
		app.dispatch({ type: 'openPage', page: 'reader' });
	}

	function parallel(id: string) {
		app.dispatch({ type: 'openInParallel', then: { type: 'addParallel', id } });
		app.dispatch({ type: 'openPage', page: 'reader' });
	}
</script>

<PageHead title="Translations" {subtitle}>
	{#snippet actions()}
		<Button kind="primary" icon="download" label="Import…" onclick={() => app.dispatch({ type: 'importTranslation' })} />
	{/snippet}
</PageHead>

<div class="filters">
	<div class="tabs" role="tablist" aria-label="Show">
		<PillTab label="All" count={all.length} selected={filter === 'all'} onclick={() => (filter = 'all')} />
		<PillTab label="Bundled" count={nb} selected={filter === 'bundled'} onclick={() => (filter = 'bundled')} />
		<PillTab label="Imported" count={ni} selected={filter === 'imported'} onclick={() => (filter = 'imported')} />
	</div>
	<span class="sorted">Sorted by</span>
	<button
		class="sort"
		bind:this={sortButton}
		aria-haspopup="menu"
		aria-expanded={sorting}
		onclick={() => (sorting = !sorting)}
	>
		{sort === 'kind' ? 'Kind' : 'Name'}<Icon name="chevron-down" size={12} />
	</button>
</div>
{#if sorting}
	<Menu anchor={sortButton ?? null} align="end" onclose={() => (sorting = false)} label="Sort by">
		{#each [['kind', 'Kind'], ['name', 'Name']] as const as [value, label] (value)}
			<MenuItem
				{label}
				checked={sort === value}
				checkColumn
				onclick={() => {
					sort = value;
					sorting = false;
				}}
			/>
		{/each}
	</Menu>
{/if}

<div class="body">
	<div class="list" role="listbox" aria-label="Translations">
		{#each shown as t (t.id)}
			{@const isSelected = t.id === selectedId}
			{@const isDefault = t.id === app.settings.translation}
			<div
				class={['row', { selected: isSelected }]}
				role="option"
				aria-selected={isSelected}
				tabindex="0"
				onclick={() => (app.translationPick = t.id)}
				ondblclick={() => read(t.id)}
				onkeydown={(e) => e.key === 'Enter' && read(t.id)}
			>
				<span class={['tile', { accent: isSelected || isDefault, long: t.abbrev.length > 3 }]}>{t.abbrev}</span>
				<span class="titles">
					<span class="name">{t.name}</span>
					<span class="description">{description(t)}</span>
				</span>
				<span class="size">{size(t)}</span>
				<span class="badges">
					{#if t.imported}<Badge kind="accent" label="Imported" />{:else}<Badge label="Bundled" />{/if}
					{#if isDefault}<Badge kind="solid" label="Default" />{/if}
				</span>
				<button
					class="more"
					aria-label="More for {t.name}"
					aria-haspopup="menu"
					onclick={(e) => {
						e.stopPropagation();
						more = more?.id === t.id ? null : { id: t.id, el: e.currentTarget };
					}}
					ondblclick={(e) => e.stopPropagation()}
				>
					<Icon name="ellipsis" size={16} heavy />
				</button>
			</div>
		{:else}
			<p class="none">No translations match.</p>
		{/each}
	</div>

	{#if selected}
		{@const isDefault = selected.id === app.settings.translation}
		<aside class="inspector" aria-label={selected.name}>
			<div class="head">
				<span class="big-tile">{selected.abbrev}</span>
				<div class="head-text">
					<div class="inspector-name">{selected.name}</div>
					<div class="facts">
						{[selected.year, LANGUAGES[selected.language] ?? selected.language, books && `${books} books`]
							.filter(Boolean)
							.join(' · ')}
					</div>
				</div>
			</div>
			<div class="inspector-badges">
				{#if selected.imported}<Badge kind="accent" label="Imported" />{:else}<Badge label="Bundled" />{/if}
				{#if selected.license.startsWith('Public Domain')}<Badge kind="outline" label="Public domain" />{/if}
				{#if selected.headings}<Badge kind="accent" label="Headings" />{/if}
				{#if selected.redLetters}<Badge kind="accent" label="Red letters" />{/if}
			</div>
			{#if sample}<p class="sample"><CapsText text={sample} /></p>{/if}
			<dl class="facts-table">
				<dt>Source</dt>
				<dd>{selected.imported ? selected.imported.format : 'eBible.org USFM'}</dd>
				<dt>Licence</dt>
				<dd>{selected.license}</dd>
				<dt>Size</dt>
				<dd>{size(selected)}</dd>
				<dt>Features</dt>
				<dd>{features(selected)}</dd>
			</dl>
			<div class="inspector-actions">
				<Button
					size="sm"
					label={isDefault ? 'Default' : 'Make Default'}
					disabled={isDefault}
					onclick={() => app.dispatch({ type: 'setTranslation', id: selected.id })}
				/>
				<Button kind="plain" size="sm" icon="compare" label="Compare" disabled={isDefault} onclick={() => parallel(selected.id)} />
			</div>
		</aside>
	{/if}
</div>

{#if more}
	{@const t = all.find((x) => x.id === more?.id)}
	{@const isDefault = t?.id === app.settings.translation}
	{#if t}
		<Menu anchor={more.el} align="end" width={180} onclose={() => (more = null)} label="More for {t.name}">
			<MenuItem label="Read" onclick={() => ((more = null), read(t.id))} />
			<MenuItem
				label="Make Default"
				disabled={isDefault}
				onclick={() => ((more = null), app.dispatch({ type: 'setTranslation', id: t.id }))}
			/>
			<MenuItem label="Read in Parallel" disabled={isDefault} onclick={() => ((more = null), parallel(t.id))} />
			{#if t.imported}
				<MenuSeparator />
				<MenuItem
					label="Remove…"
					onclick={() => ((more = null), app.dispatch({ type: 'removeTranslation', id: t.id }))}
				/>
			{/if}
		</Menu>
	{/if}
{/if}

<style>
	.filters {
		display: flex;
		align-items: center;
		height: 28px;
		margin-bottom: 20px;
	}

	.tabs {
		display: flex;
		flex: 1;
		gap: 4px;
	}

	.sorted {
		margin-right: 8px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.sort {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		color: var(--ink-2);
		font: 600 12px/16px var(--font-ui);
	}

	.sort :global(svg) {
		color: var(--ink-3);
	}

	.body {
		display: flex;
		align-items: flex-start;
		gap: 24px;
	}

	.list {
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		gap: 2px;
		min-width: 320px;
	}

	.row {
		position: relative;
		display: flex;
		align-items: center;
		height: 60px;
		padding: 0 12px;
		border-radius: 12px;
		cursor: default;
		outline: none;
	}

	.row:hover {
		background: var(--fill);
	}

	.row.selected {
		background: var(--accent-soft);
	}

	.row:focus-visible {
		box-shadow: 0 0 0 3px var(--focus);
	}

	.tile {
		display: grid;
		flex: none;
		place-items: center;
		width: 48px;
		height: 40px;
		border-radius: 10px;
		background: var(--fill);
		color: var(--ink-2);
		font: 700 12px/16px var(--font-ui);
	}

	.tile.long {
		font-size: 11px;
	}

	.tile.accent {
		color: var(--accent);
	}

	.selected .tile {
		background: var(--raised);
	}

	.titles {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-width: 0;
		margin-left: 14px;
		padding-bottom: 1px;
	}

	.name,
	.description {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.name {
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.description {
		color: var(--ink-2);
		font: var(--type-small);
	}

	.size {
		flex: none;
		width: 110px;
		margin: 0 14px;
		color: var(--ink-3);
		font: var(--type-small);
		text-align: right;
		white-space: nowrap;
	}

	.badges {
		display: flex;
		flex: none;
		justify-content: flex-end;
		gap: 6px;
		width: 150px;
		margin-right: 14px;
	}

	.more {
		display: grid;
		flex: none;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		color: var(--ink-2);
	}

	.more:hover {
		background: var(--fill);
	}

	.more:active {
		background: var(--fill-strong);
	}

	.none {
		padding: 20px 12px;
		color: var(--ink-3);
	}

	.inspector {
		flex: none;
		width: 280px;
		padding: 20px;
		border-radius: 16px;
		background: var(--sidebar);
		box-shadow: var(--shadow-panel);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.big-tile {
		display: grid;
		flex: none;
		place-items: center;
		width: 56px;
		height: 48px;
		border-radius: 12px;
		background: var(--raised);
		box-shadow: var(--shadow-control);
		color: var(--accent);
		font: 700 15px/20px var(--font-ui);
	}

	.head-text {
		min-width: 0;
	}

	.inspector-name {
		overflow: hidden;
		color: var(--ink);
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.facts {
		margin-top: 2px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.inspector-badges {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		height: 20px;
		margin-top: 16px;
		overflow: hidden;
	}

	.sample {
		display: -webkit-box;
		margin-top: 16px;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		color: var(--ink);
		font: var(--type-snippet-sm);
	}

	.facts-table {
		display: grid;
		grid-template-columns: 80px minmax(0, 1fr);
		row-gap: 8px;
		margin: 16px 0 0;
	}

	.facts-table dt {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.facts-table dd {
		overflow: hidden;
		margin: 0;
		color: var(--ink);
		font: 500 12px/16px var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.inspector-actions {
		display: flex;
		gap: 8px;
		margin-top: 16px;
	}
</style>
