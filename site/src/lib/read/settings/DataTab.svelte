<!--
	Settings › Data (settings_ui.rs `data`): what's in the library and where, exports, backups and
	erasing. The Mac keeps its library and backups as files; here they are in the browser, so the
	last backup is restored from its row rather than picked as a file.
-->
<script lang="ts">
	import { getApp, now } from '../app.svelte';
	import { capsText, cmdText, fixedCaps } from '../keyboard';
	import { clock, dayStart, shortDate } from '../time';
	import { Button, Checkbox, SelectField, Toggle } from '../ui';

	const app = getApp();
	const s = $derived(app.settings);

	let confirmErase = $state(false);

	const stats = $derived.by(() => {
		void app.studyGen;
		const c = app.counts();
		return [
			{ n: c.highlights, label: 'highlights' },
			{ n: c.bookmarks, label: 'bookmarks' },
			{ n: c.notes, label: 'notes' },
			{ n: app.plans().length, label: 'plans' }
		];
	});

	const INCLUDE = [
		{ key: 'highlights', label: 'Highlights' },
		{ key: 'notes', label: 'Notes' },
		{ key: 'bookmarks', label: 'Bookmarks' },
		{ key: 'history', label: 'History' }
	] as const;

	const hasBackup = $derived.by(() => {
		void app.studyGen;
		return app.backups().length > 0;
	});

	const lastBackup = $derived.by(() => {
		void app.studyGen;
		const last = app.backups()[0];
		if (!last) return 'Never';
		const day = dayStart(last.at);
		const today = dayStart(now());
		const when = day === today ? 'Today' : day === today - 86_400 ? 'Yesterday' : shortDate(last.at);
		return `${when} at ${clock(last.at)} · ${Math.ceil(new Blob([last.json]).size / 1024)} KB`;
	});

	const storage = $derived.by(() => {
		void app.studyGen;
		const study = app.core?.bible.snapshot() ?? '';
		const backups = app.backups().reduce((n, b) => n + b.json.length, 0);
		return `This browser · ${Math.ceil(new Blob([study]).size / 1024 + backups / 1024)} KB`;
	});
</script>

<h3 class="group-title">Your library</h3>
<div class="rows">
	<div class="row stats" style:height="72px">
		{#each stats as st (st.label)}
			<span class="stat"><span class="n">{st.n}</span><span class="what">{st.label}</span></span>
		{/each}
	</div>
	<div class="row">
		<span class="row-label">Stored in</span>
		<span class="path">{storage}</span>
	</div>
</div>

<h3 class="group-title">Export</h3>
<div class="rows">
	<div class="row" style:height="56px">
		<span class="row-label">Markdown<small>Readable notes, highlights and bookmarks · {cmdText('E')}</small></span>
		<Button size="sm" icon="upload" label="Export…" onclick={() => app.dispatch({ type: 'export', kind: 'markdown' })} />
	</div>
	<div class="row" style:height="56px">
		<span class="row-label">JSON backup<small>Everything, re-importable · {capsText(fixedCaps(['⇧', '⌘', 'E']))}</small></span>
		<Button size="sm" icon="upload" label="Export…" onclick={() => app.dispatch({ type: 'export', kind: 'json' })} />
	</div>
	<div class="row">
		<span class="row-label">Include</span>
		<span class="include">
			{#each INCLUDE as { key, label } (key)}
				<Checkbox bind:checked={s.exportInclude[key]} {label} />
			{/each}
		</span>
	</div>
</div>

<h3 class="group-title">Backup</h3>
<div class="rows">
	<div class="row">
		<span class="row-label">Back up automatically</span>
		<Toggle bind:checked={s.backupAuto} label="Back up automatically" />
	</div>
	<div class="row">
		<span class="row-label">Frequency</span>
		<SelectField
			options={[
				{ value: 'daily', label: 'Daily' },
				{ value: 'weekly', label: 'Weekly' }
			]}
			value={s.backupWeekly ? 'weekly' : 'daily'}
			onchange={(v) => (s.backupWeekly = v === 'weekly')}
			label="Frequency"
			width={160}
		/>
	</div>
	<div class="row">
		<span class="row-label">Last backup</span>
		<span class="last">{lastBackup}</span>
		{#if hasBackup}
			<Button size="sm" label="Restore" onclick={() => app.dispatch({ type: 'restoreBackup' })} />
		{/if}
	</div>
	<div class="row">
		<span class="row-label">Restore from a JSON backup</span>
		<Button size="sm" icon="download" label="Import Backup…" onclick={() => app.dispatch({ type: 'importStudy' })} />
	</div>
</div>

<div class="rows erase">
	<div class="row" style:height="56px">
		{#if confirmErase}
			<span class="row-label">Erase everything?<small>A backup is saved first; otherwise this can't be undone.</small></span>
			<Button kind="plain" size="sm" label="Cancel" onclick={() => (confirmErase = false)} />
			<Button
				kind="destructive"
				size="sm"
				label="Erase Everything"
				onclick={() => {
					app.dispatch({ type: 'eraseStudy' });
					confirmErase = false;
				}}
			/>
		{:else}
			<span class="row-label">
				Erase all study data<small>Highlights, notes, bookmarks, plans and history. Translations stay.</small>
			</span>
			<Button kind="destructive" size="sm" label="Erase…" onclick={() => (confirmErase = true)} />
		{/if}
	</div>
</div>

<style>
	.stats {
		gap: 0;
		align-items: stretch;
		padding-right: 0;
	}

	.stat {
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		padding: 14px 16px 0 0;
	}

	.n {
		color: var(--ink);
		font: 400 22px/28px var(--font-reading);
	}

	.what {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.path {
		overflow: hidden;
		color: var(--ink-2);
		font: 12px/16px ui-monospace, SFMono-Regular, Menlo, monospace;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.include {
		display: flex;
		gap: 16px;
	}

	.last {
		color: var(--ink-2);
		font: var(--type-ui);
	}

	.rows .row :global(.button + .button) {
		margin-left: -4px;
	}

	.erase {
		margin-top: 24px;
	}
</style>
