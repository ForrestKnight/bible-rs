<!--
	A plan's day (plans.rs `day_view`): a strip of the days around it, its readings to tick off, a
	reflection on its first verse, and beside them the plan's progress, what's next and the reminder.
-->
<script lang="ts">
	import { getApp, now } from '../app.svelte';
	import type { VerseRef } from '../core';
	import { weekday, weekdayShort, ago } from '../time';
	import { Button, CapsText, Icon, RoundButton, Toggle } from '../ui';
	import { sameVerse } from './notes.svelte';
	import { DAY, MONTHS, Under, minutes, openPassage, passageSize, plansPage } from './plans.svelte';

	let { under: u, day: asked }: { under: Under; day: number } = $props();

	const app = getApp();
	const len = $derived(u.len);
	const day = $derived(Math.min(Math.max(asked, 1), Math.max(len, 1)));
	const passages = $derived(u.day(day));
	const label = (d: number) => u.days[d - 1]?.label ?? '';
	const date = (d: number) => u.state.started + (d - 1) * DAY;
	const go = (d: number) => (plansPage.open = { id: u.info.id, day: d });

	const words = $derived(passages.reduce((n, p) => n + passageSize(app, p)[1], 0));
	const subtitle = $derived.by(() => {
		const dt = new Date(date(day) * 1000);
		const when = `${weekday(date(day))}, ${dt.getDate()} ${MONTHS[dt.getMonth()]}`;
		return words > 0 ? `${when} · about ${minutes(words)} minutes` : when;
	});
	const strip = $derived.by(() => {
		const start = Math.max(1, Math.min(Math.max(1, day - 4), Math.max(1, len - 8)));
		return Array.from({ length: Math.min(start + 8, len) - start + 1 }, (_, k) => start + k);
	});
	const firstOpen = $derived(passages.findIndex((_, i) => !u.passageDone(day, i)));

	const firstVerse = (p: (typeof passages)[number]) =>
		app
			.verses(app.textTranslation(), p.chapterIndex)
			?.find((v) => !p.verses || v.number >= p.verses[0])
			?.runs.map((r) => r.text)
			.join('')
			.trim() ?? '';

	// The reflection: a note on the day's first verse.
	const verse = $derived.by(() => {
		const p = passages[0];
		if (!p) return null;
		const book = app.core?.books.findLast((b) => b.firstChapter <= p.chapterIndex);
		return book ? { book: book.index, chapter: p.chapterIndex - book.firstChapter + 1, verse: p.verses?.[0] ?? 1 } : null;
	});
	const saved = $derived(verse ? (app.noteFor(verse)?.body ?? '') : '');
	const note = $derived(verse ? app.noteFor(verse) : null);
	/** What's typed and not saved yet, on the verse it's typed on: another tab saving the note keeps it. */
	let typed = $state<{ verse: VerseRef; text: string } | null>(null);
	const draft = $derived(typed && verse && sameVerse(typed.verse, verse) ? typed.text : saved);
	let editor = $state<HTMLTextAreaElement>();
	const quote = $derived(
		verse ? (app.core?.verse(app.textTranslation(), verse)?.runs.map((r) => r.text).join('').trim() ?? '') : ''
	);

	/** Ticks a reading off (or back on). Every day read finishes the plan; un-ticking one resumes it. */
	function mark(passage: number, read: boolean) {
		app.dispatch({ type: 'setPlanRead', plan: u.info.id, day, passage, read });
		const after = new Under(u.info, u.state, app);
		const complete = after.doneDays() === after.len;
		if (complete !== (u.state.finished !== null)) {
			app.dispatch({ type: 'finishPlan', id: u.info.id, at: complete ? now() : null });
		}
	}

	const done = $derived(u.doneDays());
	const frac = $derived(done / Math.max(len, 1));
	const R = 55;
	const C = 2 * Math.PI * R;
</script>

<nav class="crumb" aria-label="Breadcrumb">
	<button onclick={() => (plansPage.open = null)}>Reading Plans</button>
	<Icon name="disclosure" size={12} />
	<span>{u.info.title}</span>
</nav>

<div class="head">
	<div>
		<h1 class="title">Day {day} · {label(day)}</h1>
		<p class="subtitle">{subtitle}</p>
	</div>
	<div class="steps">
		{#if day > 1}<RoundButton icon="chevron-left" label="Previous day" size={32} class="prev" onclick={() => go(day - 1)} />{/if}
		{#if day < len}<RoundButton icon="chevron-right" label="Next day" size={32} class="next" onclick={() => go(day + 1)} />{/if}
	</div>
</div>

<div class="strip" role="tablist" aria-label="Days">
	{#each strip as d (d)}
		<button
			class={['cell', { selected: d === day, done: u.dayDone(d) }]}
			role="tab"
			aria-selected={d === day}
			aria-label="Day {d}"
			onclick={() => go(d)}
		>
			<span class={['wd', { today: d === u.today }]}>{weekdayShort(date(d))}</span>
			<span class="circle">
				{#if d !== day && u.dayDone(d)}<Icon name="check" size={12} />{:else}{d}{/if}
			</span>
		</button>
	{/each}
</div>

<div class="body">
	<div class="main">
		{#each passages as p, i (i)}
			{@const isDone = u.passageDone(day, i)}
			{@const [verses, w] = passageSize(app, p)}
			<div class={['reading', { done: isDone }]}>
				<button
					class={['tick', { on: isDone }]}
					role="checkbox"
					aria-checked={isDone}
					aria-label="Read {p.label}"
					onclick={() => mark(i, !isDone)}
				>
					{#if isDone}<Icon name="check" size={12} />{/if}
				</button>
				<div class="reading-text">
					<div class="reading-title">{p.label}</div>
					<div class="first"><CapsText text={firstVerse(p)} /></div>
				</div>
				<span class="size">{verses} verses · {minutes(w)} min</span>
				<Button
					size="sm"
					kind={!isDone && i === firstOpen ? 'primary' : 'plain'}
					label={isDone ? 'Read again' : 'Read'}
					onclick={() => openPassage(app, p)}
				/>
			</div>
		{/each}

		<h2 class="reflection-label">Reflection</h2>
		{#if verse}
			<div class="reflection">
				<div class="reflection-top">
					<Icon name="note" size={16} heavy />
					<span class="reflection-ref">{app.label(verse)}</span>
					<span class="abbrev">{app.info(app.textTranslation())?.abbrev}</span>
					<span class="when">{note ? ago(note.updated) : 'Today'}</span>
				</div>
				<p class="quote"><CapsText text={quote} /></p>
				<textarea
					bind:this={editor}
					bind:value={() => draft, (text) => verse && (typed = { verse, text })}
					rows="1"
					placeholder="Write a note…"
					aria-label="Reflection"
					style:field-sizing="content"
				></textarea>
				<div class="reflection-foot">
					<span>Markdown supported</span>
					<Button
						kind="plain"
						size="sm"
						label="Cancel"
						disabled={draft === saved}
						onclick={() => {
							typed = null;
							editor?.blur();
						}}
					/>
					<Button
						kind="primary"
						size="sm"
						label="Save"
						disabled={draft === saved}
						onclick={() => {
							if (verse) app.dispatch({ type: 'saveNote', verse, body: draft });
							typed = null;
						}}
					/>
				</div>
			</div>
		{/if}
	</div>

	<aside class="side">
		<div class="progress">
			<svg width="120" height="120" viewBox="0 0 120 120" aria-hidden="true">
				<circle cx="60" cy="60" r={R} class="ring" />
				{#if frac > 0}
					<circle
						cx="60"
						cy="60"
						r={R}
						class="arc"
						stroke-dasharray="{C * frac} {C}"
						transform="rotate(-90 60 60)"
					/>
				{/if}
			</svg>
			<span class="pct">{Math.round(frac * 100)} %</span>
			<span class="days">{done} of {len} days</span>
		</div>
		{#if day < len}
			<h3 class="up-next">Up next</h3>
			<ul class="next">
				{#each Array.from({ length: Math.min(day + 3, len) - day }, (_, k) => day + 1 + k) as d (d)}
					<li><span class="next-day">Day {d}</span>{label(d)}</li>
				{/each}
			</ul>
		{/if}
		<div class={['reminder', { alone: day >= len }]}>
			<div>
				<div class="reminder-title">Daily reminder</div>
				<div class="reminder-sub">7:00 AM · while the page is open</div>
			</div>
			<Toggle
				checked={app.settings.planReminder}
				label="Daily reminder"
				onchange={(on) => app.dispatch({ type: 'setPlanReminder', on })}
			/>
		</div>
	</aside>
</div>

<style>
	.crumb {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 16px;
		color: var(--ink-3);
		font: 500 13px/16px var(--font-ui);
	}

	.crumb button {
		color: var(--accent);
		font: inherit;
	}

	.crumb span {
		color: var(--ink-2);
	}

	.head {
		display: flex;
		justify-content: space-between;
		margin-top: 20px;
	}

	.title {
		color: var(--ink);
		font: var(--type-heading);
		font-variation-settings: 'opsz' 28;
		letter-spacing: var(--type-heading-tracking);
	}

	.subtitle {
		margin-top: 2px;
		color: var(--ink-2);
	}

	.steps {
		display: grid;
		grid-template-columns: 32px 32px;
		gap: 8px;
		margin-top: 26px;
	}

	.steps :global(.prev) {
		grid-column: 1;
	}

	.steps :global(.next) {
		grid-column: 2;
	}

	.strip {
		display: flex;
		gap: 4px;
		margin-top: 20px;
	}

	.cell {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 56px;
		height: 68px;
		padding-top: 8px;
		border-radius: 12px;
	}

	.cell:hover {
		background: var(--fill);
	}

	.cell.selected {
		background: var(--accent-soft);
	}

	.wd {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.wd.today {
		color: var(--accent);
	}

	.circle {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		margin-top: 4px;
		border-radius: 50%;
		background: var(--fill);
		color: var(--ink-2);
		font: 600 13px/16px var(--font-ui);
	}

	.done .circle {
		background: color-mix(in srgb, var(--accent) 55%, transparent);
		color: var(--on-accent);
	}

	.selected .circle {
		background: var(--accent);
		color: var(--on-accent);
	}

	.body {
		display: flex;
		align-items: flex-start;
		gap: 32px;
		margin-top: 20px;
	}

	.main {
		flex: 1 1 auto;
		min-width: 360px;
	}

	.reading {
		display: flex;
		align-items: center;
		gap: 14px;
		height: 70px;
		margin-bottom: 12px;
		padding: 0 16px;
		border-radius: 14px;
		background: var(--surface);
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.reading.done {
		opacity: 0.75;
	}

	.tick {
		display: grid;
		flex: none;
		place-items: center;
		width: 16px;
		height: 16px;
		border-radius: 4px;
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--on-accent);
	}

	.tick:hover {
		box-shadow: inset 0 0 0 1px var(--ink-3);
	}

	.tick.on {
		background: var(--accent);
		box-shadow: none;
	}

	.reading-text {
		flex: 1;
		min-width: 0;
	}

	.reading-title {
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.first {
		overflow: hidden;
		margin-top: 2px;
		color: var(--ink-2);
		font: 400 13px/20px var(--font-reading);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.size {
		flex: none;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.reflection-label {
		height: 28px;
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.reflection {
		padding: 16px;
		border-radius: 12px;
		background: var(--surface);
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.reflection:focus-within {
		box-shadow:
			0 0 0 1px var(--accent),
			0 0 0 4px var(--focus),
			var(--shadow-popover);
	}

	.reflection-top {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 16px;
		color: var(--accent);
	}

	.reflection-ref {
		margin-left: 0;
		color: var(--ink);
		font: 600 13px/16px var(--font-ui);
	}

	.abbrev {
		flex: 1;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.when {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.quote {
		margin-top: 10px;
		color: var(--ink-2);
		font: italic 400 14px/20px var(--font-reading);
	}

	textarea {
		display: block;
		width: 100%;
		min-height: 20px;
		margin-top: 10px;
		padding: 0;
		border: 0;
		outline: none;
		background: none;
		color: var(--ink);
		font: 400 14px/20px var(--font-ui);
		resize: none;
		caret-color: var(--accent);
	}

	textarea::placeholder {
		color: var(--ink-3);
	}

	.reflection-foot {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 10px;
	}

	.reflection-foot span {
		flex: 1;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.side {
		flex: none;
		width: 280px;
	}

	.progress {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 184px;
		padding-top: 20px;
		border-radius: 16px;
		background: var(--sidebar);
		box-shadow: var(--shadow-panel);
	}

	.ring,
	.arc {
		fill: none;
		stroke-width: 10;
	}

	.ring {
		stroke: var(--fill-strong);
	}

	.arc {
		stroke: var(--accent);
		stroke-linecap: round;
	}

	.pct {
		position: absolute;
		top: 40px;
		color: var(--ink);
		font: 400 28px/36px var(--font-reading);
	}

	.days {
		position: absolute;
		top: 148px;
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.up-next {
		margin-top: 16px;
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.next {
		margin: 2px 0 0;
		padding: 0;
		list-style: none;
	}

	.next li {
		display: flex;
		align-items: center;
		height: 28px;
		margin-bottom: 2px;
		color: var(--ink);
	}

	.next-day {
		flex: none;
		width: 56px;
		color: var(--ink-3);
		font: 500 12px/16px var(--font-ui);
	}

	.reminder {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 60px;
		margin-top: 14px;
		padding: 0 16px;
		border-radius: 12px;
		background: var(--surface);
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.reminder.alone {
		margin-top: 16px;
	}

	.reminder-title {
		color: var(--ink);
		font: 500 13px/20px var(--font-ui);
	}

	.reminder-sub {
		color: var(--ink-3);
		font: var(--type-small);
	}
</style>
