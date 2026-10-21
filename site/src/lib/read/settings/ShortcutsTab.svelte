<!--
	Settings › Shortcuts (settings_ui.rs `shortcuts`): every shortcut by group; a row that does one
	thing takes a new chord when its keys are clicked (taken from any row that had it).
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { GROUPS, SHORTCUTS, chordFor, fromEvent, rowCaps, spec, type Chord } from '../keyboard';
	import { Button, Keycap, RoundButton, TextField } from '../ui';

	const app = getApp();
	const overrides = $derived(app.settings.shortcuts);

	let filter = $state('');
	let recording = $state<string | null>(app.demo.record ?? null);

	const groups = $derived.by(() => {
		const q = filter.trim().toLowerCase();
		return GROUPS.map((group) => ({
			group,
			rows: SHORTCUTS.filter((s) => s.group === group && (!q || s.label.toLowerCase().includes(q)))
		})).filter((g) => g.rows.length);
	});

	const same = (a: Chord | null, b: Chord | null) => !!a && !!b && spec(a) === spec(b);

	function record(e: KeyboardEvent) {
		if (!recording) return;
		e.preventDefault();
		e.stopPropagation();
		if (e.key === 'Escape') {
			recording = null;
			return;
		}
		const chord = fromEvent(e);
		if (!chord) return;
		const next = { ...overrides };
		for (const other of SHORTCUTS) {
			if (other.id !== recording && other.chord && same(chordFor(other, overrides), chord)) next[other.id] = '';
		}
		const row = SHORTCUTS.find((s) => s.id === recording);
		if (same(row?.chord ?? null, chord)) delete next[recording];
		else next[recording] = spec(chord);
		app.settings.shortcuts = next;
		recording = null;
	}
</script>

<svelte:window onkeydowncapture={record} />

<div class="tools">
	<TextField bind:value={filter} icon="search" placeholder="Filter shortcuts" aria-label="Filter shortcuts" class="filter" />
	<Button
		label="Restore Defaults"
		onclick={() => {
			app.settings.shortcuts = {};
			recording = null;
		}}
	/>
</div>

{#each groups as { group, rows } (group)}
	<h3 class="group-title">{group}</h3>
	<div class="rows">
		{#each rows as s (s.id)}
			{@const caps = rowCaps(s, overrides)}
			<div class="row" style:height="40px">
				<span class="row-label">{s.label}</span>
				{#if recording === s.id}
					<span class="recording" role="status">Press new shortcut…</span>
					<RoundButton icon="close" label="Cancel" size={24} onclick={() => (recording = null)} />
				{:else if s.chord}
					<button type="button" class="caps change" title="Click to change" onclick={() => (recording = s.id)}>
						{#if caps.length}
							{#each caps as c, i (i)}<Keycap label={c} />{/each}
						{:else}
							<span class="none">None</span>
						{/if}
					</button>
				{:else}
					<span class="caps">{#each caps as c, i (i)}<Keycap label={c} />{/each}</span>
				{/if}
			</div>
		{/each}
	</div>
{/each}

<style>
	.tools {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 32px;
		margin-bottom: 24px;
	}

	.tools :global(.filter) {
		width: 320px;
	}

	.caps {
		display: flex;
		justify-content: flex-end;
		gap: 4px;
		min-width: 40px;
		height: 24px;
		align-items: center;
	}

	.change {
		margin: -4px;
		padding: 4px;
		box-sizing: content-box;
		border-radius: 8px;
	}

	.change:hover {
		background: var(--fill);
	}

	.none {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.recording {
		position: relative;
		width: 150px;
		height: 28px;
		margin-right: -6px;
		padding-left: 17.5px;
		border-radius: 8px;
		background: var(--field);
		box-shadow:
			inset 0 0 0 1px var(--accent),
			0 0 0 3px var(--focus);
		color: var(--accent);
		font: 500 12px/28px var(--font-ui);
	}

	.recording::before {
		content: '';
		position: absolute;
		top: 7px;
		left: 10px;
		width: 1.5px;
		height: 14px;
		background: var(--accent);
		animation: blink 1s steps(1) infinite;
	}

	@keyframes blink {
		50% {
			opacity: 0;
		}
	}
</style>
