<!--
	The keyboard shortcuts (shortcuts_overlay.rs, `?`): every shortcut in three columns with
	keycaps, a filter, and a way to change them in Settings.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { GROUPS, SHORTCUTS, rowCaps } from '../keyboard';
	import { Button, Keycap, RoundButton, Sheet, TextField } from '../ui';

	const app = getApp();

	let filter = $state('');

	const columns = $derived.by(() => {
		const q = filter.trim().toLowerCase();
		return GROUPS.map((group) => ({
			group,
			rows: SHORTCUTS.filter((s) => s.group === group && (!q || s.label.toLowerCase().includes(q)))
		}));
	});
	const tallest = $derived(Math.max(1, ...columns.map((c) => c.rows.length)));

	const close = () => (app.shortcuts = false);
</script>

<Sheet label="Keyboard Shortcuts" onclose={close} width={960} top={72} focusSheet>
	<header class="head">
		<h2 class="title">Keyboard Shortcuts</h2>
		<TextField bind:value={filter} icon="search" placeholder="Filter" aria-label="Filter shortcuts" class="filter" />
		<RoundButton icon="close" label="Close" onclick={close} />
	</header>
	<div class="body" style:--rows={tallest}>
		{#if columns.every((c) => c.rows.length === 0)}
			<p class="none">No shortcut matches.</p>
		{:else}
			{#each columns as { group, rows } (group)}
				<section class="column">
					<h3 class="group">{group}</h3>
					{#each rows as s (s.id)}
						<div class="row">
							<span class="label">{s.label}</span>
							<span class="caps">{#each rowCaps(s, app.settings.shortcuts) as c, i (i)}<Keycap label={c} />{/each}</span>
						</div>
					{/each}
				</section>
			{/each}
		{/if}
	</div>
	<footer class="foot">
		<p class="tip">Press ? any time to see this.</p>
		<Button
			kind="ghost"
			size="sm"
			icon="arrow-right"
			trailing
			label="Customize in Settings"
			onclick={() => {
				close();
				app.dispatch({ type: 'openSettings', tab: 'shortcuts' });
			}}
		/>
	</footer>
</Sheet>

<style>
	.head {
		display: flex;
		align-items: center;
		height: 60px;
		padding: 0 16px 0 24px;
		box-shadow: inset 0 -1px 0 var(--separator);
	}

	.title {
		flex: 1;
		margin: 0;
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.head :global(.filter) {
		width: 200px;
		margin-right: 12px;
	}

	.body {
		display: flex;
		gap: 32px;
		min-height: calc(20px + 21px + 32px * var(--rows) + 8px);
		padding: 20px 24px 8px;
		overflow-y: auto;
	}

	.column {
		width: 280px;
		flex-shrink: 0;
	}

	.group {
		margin: 0 0 5px;
		color: var(--accent);
		font: 600 11px/16px var(--font-ui);
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 32px;
		gap: 12px;
	}

	.label {
		color: var(--ink);
		font: var(--type-ui);
	}

	.caps {
		display: flex;
		gap: 4px;
	}

	.none {
		margin: 0;
		color: var(--ink-3);
		font: var(--type-ui);
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-shrink: 0;
		height: 44px;
		padding: 0 16px 0 24px;
		background: var(--fill);
		box-shadow: inset 0 1px 0 var(--separator);
	}

	.tip {
		margin: 0;
		color: var(--ink-3);
		font: 400 12px/16px var(--font-ui);
	}
</style>
