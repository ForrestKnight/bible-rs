<!--
	The import sheet (import_sheet.rs): choose or drop a file, watch it convert book by book, then
	review anything that needs a look before the translation joins the library. Put away while
	converting (Escape, or a click beside it), it returns when the conversion ends.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Badge, Button, Icon, Sheet, TextField } from '../ui';
	import { importer, issueBadge, issueBody, issueTitle, summary } from './importer.svelte';

	const STATUS = ['Reading the file…', 'Detecting the format…', 'Converting books…', 'Checking the text…'];

	const app = getApp();
	const im = importer;
	const peek = $derived(im.peek);
	const label = $derived(peek?.label ?? im.picked?.label ?? '');
	const found = $derived(im.issues);
	const names = $derived(app.core?.books.map((b) => b.name) ?? []);

	const title = $derived(
		im.stage === 'choose'
			? 'Import a Translation'
			: im.stage === 'converting'
				? `Importing ${im.displayName}`
				: im.result
					? 'Import finished with problems'
					: 'Import didn’t finish'
	);

	// Converting: a clock for the time left and the steps' durations.
	let clock = $state(performance.now());
	$effect(() => {
		if (im.stage !== 'converting') return;
		const t = setInterval(() => (clock = performance.now()), 100);
		return () => clearInterval(t);
	});

	const step = $derived(im.progress.step);
	const books = $derived(Math.min(im.progress.books, 66));
	const current = $derived(im.progress.book >= 0 ? (names[im.progress.book] ?? '') : '');
	const left = $derived.by(() => {
		const t = im.started[2];
		if (t === undefined || books < 3 || books >= 66) return 'Estimating…';
		const secs = Math.max(1, Math.ceil(((clock - t) / 1000 / books) * (66 - books)));
		return secs < 60 ? `About ${secs} ${secs === 1 ? 'second' : 'seconds'} left` : `About ${Math.ceil(secs / 60)} min left`;
	});
	const took = (i: number) => {
		const [a, b] = [im.started[i], im.started[i + 1]];
		return a !== undefined && b !== undefined ? `${Math.max(0.1, (b - a) / 1000).toFixed(1)} s` : '';
	};
	const rows = $derived.by(() => {
		const sword = !!peek?.sword;
		const versification = peek?.details.find((d) => d.endsWith(' versification'))?.replace(/ versification$/, '');
		return [
			[sword ? 'Read module' : 'Read file', took(0)],
			[sword ? 'Detect versification' : 'Detect format', versification ?? peek?.format ?? ''],
			['Convert books', `${books} / 66`],
			['Check the text', ''],
			['Add to the library', '']
		];
	});

	// Review.
	const headline = $derived(im.result ? `${66 - im.result.missing.length} of 66 books imported` : 'Nothing was imported');
	const sub = $derived(
		im.result
			? `${label} · ${found.length} ${found.length === 1 ? 'issue needs' : 'issues need'} a look before ${im.result.info.abbrev} is ready to read.`
			: `${label} · fix the problem below and try again.`
	);
	const locked = $derived(found[0]?.kind === 'locked');

	function onclose() {
		if (im.stage === 'converting') im.hide(app);
		else im.close(app);
	}

	function keydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && im.stage === 'choose' && im.ready && !(e.target instanceof HTMLButtonElement)) {
			e.preventDefault();
			void im.start(app);
		}
	}

	function copyReport() {
		const report = found.map((i) => `- ${issueTitle(i)}: ${issueBody(i, names)}`).join('\n');
		void navigator.clipboard?.writeText(`${label}\n${headline}\n${report}`);
	}
</script>

<Sheet
	label={title}
	{onclose}
	width={560}
	top={im.stage === 'review' ? 56 : 72}
	dismissable={im.stage === 'converting'}
	focusSheet
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="wrap" onkeydown={keydown}>
		<header class="head"><h2 class="title">{title}</h2></header>

		{#if im.stage === 'choose'}
			<div class="body">
				<div class={['zone', { dragging: im.dragging }]}>
					<svg class="dash" aria-hidden="true"><rect x="0.75" y="0.75" rx="11.25" /></svg>
					<span class="disc"><Icon name="download" size={28} /></span>
					<p class="zone-title">
						{im.dragging ? 'Drop to import' : im.picked ? `“${label}”` : 'Drop a Bible file here'}
					</p>
					<p class={['zone-line', { danger: !im.dragging && im.peekError }]}>
						{#if im.dragging || !im.picked}
							USFM (.usfm, .sfm, folder, .zip) · OSIS XML · Zefania XML · SWORD module
						{:else if im.peekError}
							{im.peekError}
						{:else if peek}
							{summary(peek)}
						{:else}
							Reading…
						{/if}
					</p>
					<Button
						kind="secondary"
						size="sm"
						label={im.picked ? 'Choose Another…' : 'Choose File…'}
						onclick={() => im.pick()}
					/>
				</div>

				<div class="names">
					<label class="named abbrev">
						<span class="label">Abbreviation</span>
						<TextField bind:value={im.abbrev} placeholder="ABBR" />
					</label>
					<label class="named">
						<span class="label">Name</span>
						<TextField bind:value={im.name} placeholder="Translation name" />
					</label>
				</div>

				<label class="named key">
					<span class="label">
						SWORD unlock key
						<span class={['hint', { locked: peek?.locked }]}>
							{peek?.locked ? 'this module is locked — enter the key you received' : 'optional — for locked modules you purchased'}
						</span>
					</span>
					<TextField bind:value={im.key} placeholder="XXXX-XXXX-XXXX-XXXX" icon="lock" mono />
				</label>

				<p class="privacy">
					<Icon name="info" size={14} heavy />
					<span>The text is converted and stored only in this browser. Copyrighted translations are never uploaded.</span>
				</p>
			</div>
			<footer class="foot">
				<Button kind="plain" label="Cancel" onclick={() => im.close(app)} />
				<Button kind="primary" label="Import" disabled={!im.ready} onclick={() => im.start(app)} />
			</footer>
		{:else if im.stage === 'converting'}
			<div class="body">
				<div class="file">
					<span class="tile"><Icon name="doc" size={20} /></span>
					<span class="file-text">
						<span class="file-name">{label}</span>
						<span class="file-meta">{peek ? summary(peek) : ''}</span>
					</span>
					<Badge kind="accent" label={im.abbrev} />
				</div>

				<div class="status">
					<span class="status-text">{STATUS[Math.min(step, 3)]}</span>
					{#if step >= 2}<span class="count">{books} of 66{current ? ` · ${current}` : ''}</span>{/if}
				</div>
				<div class="track"><div class="fill" style:width="{step < 2 ? 0 : Math.floor((books / 66) * 512)}px"></div></div>
				<p class="left">{left}</p>

				<ol class="steps">
					{#each rows as [name, value], i (name)}
						{@const state = i < step ? 'done' : i === step ? 'current' : 'todo'}
						<li class={['row', state]}>
							<span class="dot">{#if state === 'done'}<Icon name="check" size={10} heavy />{/if}</span>
							<span class="row-name">{name}</span>
							{#if value && (state === 'done' || (state === 'current' && i === 2))}<span class="row-value">{value}</span>{/if}
						</li>
					{/each}
				</ol>
			</div>
			<footer class="foot">
				<span class="keep">You can keep reading while this finishes.</span>
				<Button kind="plain" label="Cancel Import" onclick={() => im.cancel(app)} />
			</footer>
		{:else}
			<div class="body">
				<div class="summary">
					<span class="alert"><Icon name="warning" size={24} /></span>
					<span class="summary-text">
						<span class="headline">{headline}</span>
						<span class="sub">{sub}</span>
					</span>
				</div>

				<div class="issues">
					{#each found as issue, i (issue.kind)}
						{@const [kind, badge] = issueBadge(issue)}
						{@const danger = kind === 'danger'}
						<div class={['issue', { danger }]}>
							<span class={['issue-icon', { second: i === 1 }]}>
								<Icon name={danger ? 'warning' : 'info'} size={16} heavy />
							</span>
							<div class="issue-text">
								<div class="issue-head">
									<span class="issue-title">{issueTitle(issue)}</span>
									<Badge {kind} label={badge} />
								</div>
								<p class="issue-body">{issueBody(issue, names)}</p>
								{#if issue.kind === 'locked'}
									<div class="unlock">
										<TextField
											bind:value={im.key}
											placeholder="XXXX-XXXX-XXXX-XXXX"
											icon="lock"
											mono
											error={im.keyTried !== null}
										/>
									</div>
									{#if im.keyTried !== null}
										<p class="key-error"><Icon name="warning" size={12} />That key didn’t unlock the module</p>
									{/if}
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
			<footer class="foot">
				<Button kind="ghost" icon="copy" label="Copy Report" class="report" onclick={copyReport} />
				{#if im.result}
					{@const result = im.result}
					<Button kind="secondary" label="Discard" onclick={() => im.close(app)} />
					<Button kind="primary" label="Add to Library" onclick={() => im.install(app, result)} />
				{:else}
					<Button
						kind="secondary"
						label={locked ? 'Cancel' : 'Choose Another File'}
						onclick={() => (locked ? im.close(app) : im.again())}
					/>
					<Button
						kind="primary"
						icon="retry"
						label="Try Again"
						disabled={locked && !im.key.trim()}
						onclick={() => im.start(app)}
					/>
				{/if}
			</footer>
		{/if}
	</div>
</Sheet>

<style>
	.wrap {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.head {
		display: flex;
		flex: none;
		align-items: center;
		height: 52px;
		padding: 0 24px;
		box-shadow: inset 0 -1px var(--separator);
	}

	.title {
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.body {
		display: flex;
		flex-direction: column;
		min-height: 0;
		padding: 24px;
		overflow-y: auto;
	}

	.foot {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: flex-end;
		gap: 8px;
		height: 64px;
		padding: 0 24px;
		border-radius: 0 0 16px 16px;
		background: var(--fill);
		box-shadow: inset 0 1px var(--separator);
	}

	/* Choose */

	.zone {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 199px;
		padding-top: 29.5px;
		border-radius: 12px;
		background: var(--fill);
	}

	.zone.dragging {
		background: var(--accent-soft);
	}

	.dash {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}

	.dash rect {
		width: calc(100% - 1.5px);
		height: calc(100% - 1.5px);
		fill: none;
		stroke: var(--border);
		stroke-width: 1.5;
		stroke-dasharray: 3 1.5;
	}

	.dragging .dash rect {
		stroke: var(--accent);
	}

	.disc {
		display: grid;
		place-items: center;
		width: 56px;
		height: 56px;
		border-radius: 50%;
		background: var(--surface);
		color: var(--accent);
	}

	.dragging .disc {
		background: var(--raised);
	}

	.zone-title {
		margin-top: 8px;
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.dragging .zone-title {
		color: var(--accent);
	}

	.zone-line {
		max-width: calc(100% - 48px);
		margin: 8px 0;
		overflow: hidden;
		color: var(--ink-2);
		font: var(--type-small);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.zone-line.danger {
		color: var(--danger);
	}

	.names {
		display: flex;
		gap: 12px;
		margin-top: 20px;
	}

	.named {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
	}

	.named.abbrev {
		flex: 0 0 140px;
	}

	.label {
		color: var(--ink-2);
		font: 600 12px/16px var(--font-ui);
	}

	.key {
		margin-top: 20px;
	}

	.hint {
		margin-left: 8px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.hint.locked {
		color: var(--accent);
	}

	.privacy {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 32px;
		margin-top: 20px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.privacy :global(svg) {
		flex: none;
	}

	/* Converting */

	.file {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 64px;
		padding: 0 12px;
		border-radius: 12px;
		background: var(--fill);
	}

	.tile {
		display: grid;
		flex: none;
		place-items: center;
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent);
	}

	.file-text {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-width: 0;
	}

	.file-name,
	.file-meta {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-name {
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.file-meta {
		color: var(--ink-2);
		font: var(--type-small);
	}

	.status {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin-top: 20px;
	}

	.status-text {
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.count {
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.track {
		height: 6px;
		margin-top: 8px;
		overflow: hidden;
		border-radius: 3px;
		background: var(--fill-strong);
	}

	.fill {
		height: 100%;
		border-radius: 3px;
		background: var(--accent);
	}

	.left {
		margin-top: 8px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.steps {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin: 20px 0 0;
		padding: 0;
		list-style: none;
	}

	.row {
		display: flex;
		align-items: center;
		height: 32px;
		color: var(--ink);
		font: var(--type-ui);
	}

	.row.todo {
		color: var(--ink-3);
	}

	.row.current {
		font-weight: 600;
	}

	.dot {
		display: grid;
		flex: none;
		place-items: center;
		width: 20px;
		height: 20px;
		margin-right: 12px;
		border-radius: 50%;
		box-shadow: inset 0 0 0 1.5px var(--border);
		color: var(--on-accent);
	}

	.done .dot {
		background: var(--success);
		box-shadow: none;
	}

	.current .dot {
		background: radial-gradient(circle, var(--accent) 4px, transparent 4.5px);
		box-shadow: inset 0 0 0 2px var(--accent);
	}

	.row-name {
		flex: 1;
	}

	.row-value {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.keep {
		margin-right: auto;
		color: var(--ink-3);
		font: var(--type-small);
	}

	/* Review */

	.summary {
		display: flex;
		gap: 14px;
		height: 44px;
	}

	.alert {
		display: grid;
		flex: none;
		place-items: center;
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: color-mix(in srgb, var(--danger) 12%, transparent);
		color: var(--danger);
	}

	.summary-text {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		padding-top: 3px;
	}

	.headline {
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.sub {
		overflow: hidden;
		color: var(--ink-2);
		font: var(--type-small);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.issues {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 20px;
	}

	.issue {
		display: flex;
		gap: 10px;
		padding: 12px 14px;
		border-radius: 12px;
		background: var(--fill);
	}

	.issue.danger {
		background: color-mix(in srgb, var(--danger) 7%, transparent);
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--danger) 30%, transparent);
	}

	.issue-icon {
		flex: none;
		height: 16px;
		margin-top: 2px;
		color: var(--ink-3);
	}

	.issue-icon.second {
		color: var(--ink-2);
	}

	.danger .issue-icon {
		color: var(--danger);
	}

	.issue-text {
		flex: 1;
		min-width: 0;
	}

	.issue-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		height: 20px;
	}

	.issue-title {
		overflow: hidden;
		color: var(--ink);
		font: var(--type-ui-strong);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.issue-body {
		margin-top: 6px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.unlock {
		width: 300px;
		margin-top: 6px;
	}

	.key-error {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 6px;
		color: var(--danger);
		font: var(--type-small);
	}

	.foot :global(.report) {
		margin-right: auto;
	}
</style>
