<!--
	Settings (settings_ui.rs): the Mac's 720 pt window as a sheet, its title over six icon tabs and
	the tab's groups of rows below. The row styles here are shared by the tabs.
-->
<script lang="ts">
	import { getApp, type SettingsTab } from '../app.svelte';
	import { Icon, RoundButton, Sheet, type IconName } from '../ui';
	import AboutTab from './AboutTab.svelte';
	import AppearanceTab from './AppearanceTab.svelte';
	import DataTab from './DataTab.svelte';
	import ReadingTab from './ReadingTab.svelte';
	import ShortcutsTab from './ShortcutsTab.svelte';
	import TranslationsTab from './TranslationsTab.svelte';

	const app = getApp();

	const TABS: { tab: SettingsTab; label: string; icon: IconName }[] = [
		{ tab: 'reading', label: 'Reading', icon: 'text-size' },
		{ tab: 'appearance', label: 'Appearance', icon: 'sun' },
		{ tab: 'translations', label: 'Translations', icon: 'book' },
		{ tab: 'shortcuts', label: 'Shortcuts', icon: 'keyboard' },
		{ tab: 'data', label: 'Data', icon: 'tray' },
		{ tab: 'about', label: 'About', icon: 'info' }
	];

	const tab = $derived(app.settingsTab ?? 'reading');
	const title = $derived(TABS.find((t) => t.tab === tab)?.label ?? 'Settings');
	let body = $state<HTMLElement>();

	const close = () => (app.settingsTab = null);

	function pick(t: SettingsTab) {
		app.settingsTab = t;
		body?.scrollTo({ top: 0 });
	}

	function tabKey(e: KeyboardEvent) {
		const i = TABS.findIndex((t) => t.tab === tab);
		const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
		if (!d) return;
		e.preventDefault();
		const next = TABS[(i + d + TABS.length) % TABS.length].tab;
		pick(next);
		(e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-tab="${next}"]`)?.focus();
	}
</script>

<Sheet label="Settings" onclose={close} width={720} height={860} radius={12} focusSheet class="settings">
	<header class="head">
		<RoundButton icon="close" label="Close Settings" onclick={close} class="close" />
		<h2 class="title">{title}</h2>
		<div class="tabs" role="tablist" aria-label="Settings" tabindex="-1" onkeydown={tabKey}>
			{#each TABS as t (t.tab)}
				<button
					type="button"
					class={['tab', { on: t.tab === tab }]}
					role="tab"
					aria-selected={t.tab === tab}
					tabindex={t.tab === tab ? 0 : -1}
					data-tab={t.tab}
					onclick={() => pick(t.tab)}
				>
					<Icon name={t.icon} size={20} />
					<span>{t.label}</span>
				</button>
			{/each}
		</div>
	</header>
	<div class="body" bind:this={body} role="tabpanel" aria-label={title}>
		{#if tab === 'reading'}<ReadingTab />
		{:else if tab === 'appearance'}<AppearanceTab />
		{:else if tab === 'translations'}<TranslationsTab />
		{:else if tab === 'shortcuts'}<ShortcutsTab />
		{:else if tab === 'data'}<DataTab />
		{:else}<AboutTab />{/if}
	</div>
</Sheet>

<style>
	.head {
		position: relative;
		flex: none;
		height: 116px;
		background: linear-gradient(var(--toolbar), var(--toolbar)), var(--paper);
		box-shadow: inset 0 -1px var(--separator);
	}

	.head :global(.close) {
		position: absolute;
		top: 12px;
		left: 12px;
	}

	.title {
		height: 52px;
		color: var(--ink);
		font: var(--type-ui-strong);
		line-height: 52px;
		text-align: center;
	}

	.tabs {
		display: flex;
		justify-content: center;
		gap: 4px;
		outline: none;
	}

	.tab {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		width: 76px;
		height: 52px;
		padding-top: 7px;
		border-radius: 10px;
		color: var(--ink-2);
		font: 500 11px/16px var(--font-ui);
	}

	.tab:hover:not(.on) {
		background: var(--fill);
	}

	.tab.on {
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 600;
	}

	.body {
		flex: 1;
		min-height: 0;
		padding: 24px 32px 32px;
		overflow-y: auto;
		overscroll-behavior: contain;
		background: var(--sidebar);
	}

	/* Shared by the tabs. */

	/* A group: a 13 pt semibold title, its content 8 pt below; groups are 24 pt apart. */
	.body :global(.group-title) {
		margin-bottom: 8px;
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.body :global(.group-title:not(:first-child)) {
		margin-top: 24px;
	}

	/* A card of rows: the surface with a hairline ring, radius 12; hairlines between rows inset 16. */
	.body :global(.rows) {
		border-radius: 12px;
		background: var(--surface);
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.body :global(.row) {
		position: relative;
		display: flex;
		align-items: center;
		gap: 12px;
		height: 44px;
		padding: 0 16px;
	}

	.body :global(.row + .row) {
		margin-top: 1px;
	}

	.body :global(.row + .row::before) {
		content: '';
		position: absolute;
		top: -1px;
		right: 0;
		left: 16px;
		height: 1px;
		background: var(--separator);
	}

	.body :global(.row-label) {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-width: 0;
		color: var(--ink);
		font: var(--type-ui);
	}

	.body :global(.row-label small) {
		color: var(--ink-3);
		font: var(--type-small);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.body :global(.value) {
		color: var(--ink-2);
		font: 500 13px/20px var(--font-ui);
		white-space: nowrap;
	}

	/* A note under a group: 12 pt tertiary, 8 pt below, aligned with the group's caption. */
	.body :global(.footnote) {
		margin-top: 8px;
		padding-left: 4px;
		color: var(--ink-3);
		font: var(--type-small);
	}
</style>
