<!--
	Settings › Translations (settings_ui.rs `translations`): the default and parallel set, what's
	installed, and importing a translation you own. Of the Mac's online translations the web has
	only the NLT, read live from the translation menu (../nlt.ts, DECISIONS.md).
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import type { TranslationInfo } from '../core';
	import { cmdText } from '../keyboard';
	import { Badge, Button, Menu, MenuItem, SelectField } from '../ui';

	const app = getApp();
	const s = $derived(app.settings);

	let showAll = $state(false);
	let editing = $state(false);
	let editButton = $state<HTMLButtonElement>();

	const abbrevOf = (id: string) => app.info(id)?.abbrev ?? id.toUpperCase();
	const options = $derived(app.translations.map((t) => ({ value: t.id, label: `${t.name} (${t.abbrev})` })));

	const bundled = $derived(app.translations.filter((t) => !t.imported));
	const imported = $derived(app.translations.filter((t) => t.imported));
	/** The default and parallel ones, made up to three from the top of the bundled list. */
	const shown = $derived.by(() => {
		const featured = (t: TranslationInfo) => t.id === s.translation || s.parallel.includes(t.id);
		const out = bundled.filter(featured);
		for (const t of bundled) {
			if (out.length >= 3) break;
			if (!out.includes(t)) out.push(t);
		}
		return bundled.filter((t) => out.includes(t));
	});
	const folded = $derived(bundled.filter((t) => !shown.includes(t)));
	const list = $derived(showAll ? bundled : shown);

	function detail(t: TranslationInfo): string {
		if (t.imported) return `${t.imported.format} · ${(t.imported.bytes / 1_048_576).toFixed(1)} MB`;
		// "Red letters · modern English" → "red letters, modern English".
		const what = (t.description ?? '')
			.split(' · ')
			.filter(Boolean)
			.slice(0, 2)
			.map((w) => w[0].toLowerCase() + w.slice(1))
			.join(', ');
		return [t.year, what].filter(Boolean).join(' · ');
	}

	function toggleParallel(id: string) {
		if (s.parallel.includes(id)) app.dispatch({ type: 'removeParallel', id });
		else app.dispatch({ type: 'addParallel', id });
	}
</script>

<h3 class="group-title">Reading</h3>
<div class="rows">
	<div class="row">
		<span class="row-label">Default translation</span>
		<SelectField
			{options}
			value={s.translation}
			onchange={(id) => app.dispatch({ type: 'setTranslation', id })}
			label="Default translation"
			width={260}
		/>
	</div>
	<div class="row">
		<span class="row-label">Parallel columns</span>
		<span class="value">{[s.translation, ...s.parallel].map(abbrevOf).join(' · ')}</span>
		<Button
			size="sm"
			label="Edit…"
			bind:el={editButton}
			aria-haspopup="menu"
			aria-expanded={editing}
			onclick={() => (editing = !editing)}
		/>
	</div>
</div>
{#if editing}
	<Menu anchor={editButton ?? null} align="end" width={240} onclose={() => (editing = false)} label="Parallel columns">
		{#each app.translations.filter((t) => t.id !== s.translation) as t (t.id)}
			<MenuItem
				label="{t.abbrev} — {t.name}"
				checked={s.parallel.includes(t.id)}
				checkColumn
				onclick={() => toggleParallel(t.id)}
			/>
		{/each}
	</Menu>
{/if}

<h3 class="group-title">Installed</h3>
<div class="rows">
	{#snippet entry(t: TranslationInfo)}
		{@const isDefault = t.id === s.translation}
		<div class="row" style:height="52px">
			<span class={['tile', { default: isDefault }]}>{t.abbrev}</span>
			<span class="row-label">
				<span class="name">{t.name}</span>
				<small>{detail(t)}</small>
			</span>
			{#if t.imported}<Badge kind="accent" label="Imported" />{:else}<Badge label="Bundled" />{/if}
			{#if isDefault}<Badge kind="solid" label="Default" />{/if}
			{#if t.imported}
				<Button kind="plain" size="sm" label="Remove" onclick={() => app.dispatch({ type: 'removeTranslation', id: t.id })} />
			{/if}
		</div>
	{/snippet}
	{#each list as t (t.id)}{@render entry(t)}{/each}
	{#if folded.length}
		<div class="row" style:height="40px">
			<span class="more">
				{showAll ? 'Showing every bundled translation' : `${folded.map((t) => t.abbrev).join(', ')} — ${folded.length} more bundled`}
			</span>
			<Button kind="ghost" size="sm" label={showAll ? 'Show Fewer' : 'Show All'} onclick={() => (showAll = !showAll)} />
		</div>
	{/if}
	{#each imported as t (t.id)}{@render entry(t)}{/each}
</div>

<h3 class="group-title">Import</h3>
<div class="rows">
	<div class="row" style:height="56px">
		<span class="row-label">
			Add a translation you own<small>USFM, OSIS, Zefania or SWORD · {cmdText('I')}</small>
		</span>
		<Button icon="doc" label="Import File…" onclick={() => app.dispatch({ type: 'importTranslation' })} />
		<Button icon="folder" label="Import Folder…" onclick={() => app.dispatch({ type: 'importTranslation', folder: true })} />
	</div>
</div>
<p class="footnote">Converted text is stored only in this browser. Copyrighted translations are never bundled.</p>

<style>
	.rows .row :global(.button + .button) {
		margin-left: -4px;
	}

	.tile {
		display: grid;
		place-items: center;
		flex: none;
		width: 44px;
		height: 32px;
		border-radius: 8px;
		background: var(--fill);
		color: var(--ink-2);
		font: 700 11px/16px var(--font-ui);
	}

	.tile.default {
		color: var(--accent);
	}

	.name {
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.more {
		flex: 1;
		color: var(--ink-3);
		font: var(--type-small);
	}
</style>
