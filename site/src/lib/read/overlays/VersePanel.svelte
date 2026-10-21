<!--
	The inspector beside parallel columns (panel.rs): the selected verse in every translation, the
	reading one picked first, those shown in parallel badged, and a way to add the picked one.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Badge, Icon, Menu, MenuItem, RoundButton } from '../ui';

	let { top, width }: { top: number; width: number } = $props();

	const app = getApp();

	/** The translation picked in the list (the one "Add to parallel" adds). */
	let picked = $state<string | null>(null);
	let addButton = $state<HTMLButtonElement>();
	let menu = $state(false);

	const verse = $derived(app.panel!);
	const reference = $derived(app.label(verse));
	const columns = $derived(app.columns());
	const pick = $derived(picked ?? app.settings.translation);
	const rows = $derived(
		app.translations.map((t) => {
			const v = app.loaded.has(t.id) ? app.core?.verse(t.id, verse) : undefined;
			return {
				id: t.id,
				abbrev: t.abbrev,
				name: shortName(t.name),
				runs:
					v === undefined
						? [{ text: 'Loading…', red: false }]
						: v
							? v.runs.map((r) => ({ text: r.text, red: r.red && app.settings.typo.redLetters }))
							: [{ text: '— (not in this translation)', red: false }],
				text: v ? v.runs.map((r) => r.text).join('') : null
			};
		})
	);
	const others = $derived(app.translations.filter((t) => !columns.includes(t.id)));
	const direct = $derived(!columns.includes(pick) && app.translations.some((t) => t.id === pick));
	const enabled = $derived(direct || others.length > 0);

	/** "King James Version" → "King James": names without their generic last word. */
	function shortName(name: string) {
		for (const suffix of [' Version', ' Bible', ' Translation']) {
			if (name.endsWith(suffix)) return name.slice(0, -suffix.length);
		}
		return name;
	}

	async function copyAll() {
		const lines = rows.filter((r) => r.text !== null).map((r) => `${r.abbrev}: ${r.text}`);
		try {
			await navigator.clipboard.writeText(`${reference}\n\n${lines.join('\n')}`);
			app.toastNow(`Copied ${reference} in every translation`);
		} catch {
			app.toastNow("Couldn't copy");
		}
	}

	function add() {
		if (direct) app.dispatch({ type: 'addParallel', id: pick });
		else if (enabled) menu = true;
	}
</script>

<aside class="panel" style:top="{top}px" style:width="{width - 8}px" aria-label="Compare {reference}">
	<header class="head">
		<div class="titles">
			<h2 class="title">Compare</h2>
			<p class="detail">{reference} · {app.translations.length} translations</p>
		</div>
		<RoundButton icon="copy" label="Copy every translation" onclick={copyAll} />
		<RoundButton icon="close" label="Close" onclick={() => (app.panel = null)} />
	</header>
	<div class="list" role="listbox" aria-label="Translations">
		{#each rows as r (r.id)}
			{@const selected = r.id === pick}
			<div
				class={['row', { selected }]}
				role="option"
				tabindex="0"
				aria-selected={selected}
				onclick={() => (picked = r.id)}
				onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), (picked = r.id))}
			>
				<div class="line">
					<span class="abbrev">{r.abbrev}</span>
					<span class="name">{r.name}</span>
					{#if columns.includes(r.id)}<span class="badge"><Badge kind="accent" label="In parallel" /></span>{/if}
				</div>
				<p class="text">{#each r.runs as run, i (i)}{#if run.red}<span class="red">{run.text}</span>{:else}{run.text}{/if}{/each}</p>
			</div>
		{/each}
	</div>
	<footer class="foot">
		<button
			bind:this={addButton}
			type="button"
			class="add"
			disabled={!enabled}
			aria-haspopup={direct ? undefined : 'menu'}
			onclick={add}
		>
			<Icon name="plus" size={16} heavy />Add to parallel
		</button>
		<span class="key">C</span>
	</footer>
</aside>

{#if menu && addButton}
	<Menu anchor={addButton} side="top" onclose={() => (menu = false)} label="Add to parallel">
		{#each others as t (t.id)}
			<MenuItem
				label="{t.abbrev}  {t.name}"
				onclick={() => {
					menu = false;
					app.dispatch({ type: 'addParallel', id: t.id });
				}}
			/>
		{/each}
	</Menu>
{/if}

<style>
	.panel {
		position: absolute;
		right: 8px;
		bottom: 8px;
		display: flex;
		flex-direction: column;
		border-radius: var(--radius-lg);
		background: var(--sidebar);
		box-shadow: var(--shadow-panel);
		overflow: hidden;
		z-index: 3;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
		height: 66px;
		padding: 0 12px 0 16px;
		box-shadow: inset 0 -1px 0 var(--separator);
	}

	.titles {
		flex: 1;
		min-width: 0;
		align-self: flex-start;
		padding-top: 16px;
	}

	.title {
		margin: 0;
		color: var(--ink);
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
	}

	.detail {
		margin: 2px 0 0;
		overflow: hidden;
		color: var(--ink-2);
		font: var(--type-small);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.list {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px;
		overflow-y: auto;
	}

	.row {
		flex-shrink: 0;
		padding: 10px 8px;
		border-radius: 10px;
		cursor: default;
	}

	.row:hover:not(.selected) {
		background: var(--fill);
	}

	.row.selected {
		background: var(--accent-soft);
	}

	.line {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 16px;
	}

	.abbrev {
		color: var(--ink);
		font: 600 12px/16px var(--font-ui);
	}

	.selected .abbrev {
		color: var(--accent);
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: var(--ink-3);
		font: 400 11px/16px var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.text {
		display: -webkit-box;
		margin: 4px 0 0;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 4;
		line-clamp: 4;
		color: var(--ink);
		font: 400 14px/20px var(--font-reading);
	}

	.red {
		color: var(--rubric);
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-shrink: 0;
		height: 48px;
		padding: 0 16px;
		box-shadow: inset 0 1px 0 var(--separator);
	}

	.add {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 24px;
		padding: 0 10px;
		border-radius: 12px;
		background: var(--raised);
		box-shadow: var(--shadow-control);
		color: var(--ink);
		font: 500 12px/16px var(--font-ui);
	}

	.add:hover:not(:disabled) {
		background-image: linear-gradient(var(--fill), var(--fill));
	}

	.add:disabled {
		opacity: 0.4;
	}

	.key {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}
</style>
