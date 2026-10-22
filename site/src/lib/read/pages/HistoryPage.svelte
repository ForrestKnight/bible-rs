<!--
	History (pages/history.rs): every chapter read, newest first, as a timeline under day labels —
	when, how long, the chapter's opening and how far you got — beside a card of the week's minutes.
-->
<script lang="ts">
	import { getApp, now } from '../app.svelte';
	import { chapterLabel, type ReadingSession } from '../core';
	import { clock, dayStart, duration, shortDate, weekday, weekdayShort } from '../time';
	import { Badge, Button, CapsText, Menu, MenuItem, MenuSeparator, Segmented } from '../ui';
	import PageEmpty from './PageEmpty.svelte';
	import PageHead from './PageHead.svelte';

	const DAY = 86_400;
	const app = getApp();
	let range = $state(1);
	let clearing = $state(false);
	let clearButton = $state<HTMLButtonElement>();
	let hovered = $state<number | null>(null);
	let tipWidth = $state(0);

	const today = $derived(dayStart(now()));
	const ever = $derived(app.readingSince(0));
	const week = $derived(app.readingSince(today - 6 * DAY));

	const indexOf = (s: ReadingSession) => (app.core?.books[s.book]?.firstChapter ?? 0) + s.chapter - 1;
	const title = (s: ReadingSession) => chapterLabel(indexOf(s));
	const finished = (s: ReadingSession) => s.verses > 0 && s.furthest >= s.verses;
	const progress = (s: ReadingSession) => (finished(s) ? 1 : s.verses ? Math.min(1, s.furthest / s.verses) : 0);

	const sessions = $derived.by(() => {
		const since = range === 0 ? today : range === 1 ? today - 6 * DAY : 0;
		const q = app.pageQuery.trim().toLowerCase();
		return app.readingSince(since).filter((s) => !q || title(s).toLowerCase().includes(q));
	});

	/** Where a session's opening line comes from: its translation, or for a live one, which holds
	 *  only the chapter being read, the translation search and the study lists use (`fallback`). */
	const textOf = (id: string) => (app.isLive(id) ? app.fallback() : id);

	$effect(() => {
		for (const id of new Set(sessions.map((s) => textOf(s.translation)))) void app.ensure(id);
	});

	const snippet = (s: ReadingSession) =>
		app.verses(textOf(s.translation), indexOf(s))?.[0]?.runs.map((r) => r.text).join('').trim() ?? '';

	function dayLabel(s: ReadingSession): string {
		const d = dayStart(s.started);
		if (d === today) return `Today · ${weekday(s.started)}`;
		if (d >= today - 6 * DAY) return weekday(s.started);
		return `${weekday(s.started)} · ${shortDate(s.started)}`;
	}

	function open(s: ReadingSession) {
		app.dispatch({ type: 'setTranslation', id: s.translation });
		app.dispatch({ type: 'jump', chapter: indexOf(s) });
	}

	function clear(since: number) {
		clearing = false;
		app.dispatch({ type: 'clearReading', since });
	}

	// The week: minutes a day for the last seven, oldest first.
	const days = $derived(
		[6, 5, 4, 3, 2, 1, 0].map((k) => {
			const d = today - k * DAY;
			const list = week.filter((s) => dayStart(s.started) === d);
			const minutes = Math.floor((list.reduce((n, s) => n + s.seconds, 0) + 30) / 60);
			return { d, list, minutes };
		})
	);
	const max = $derived(Math.max(1, ...days.map((d) => d.minutes)));
	const stats = $derived([
		[duration(week.reduce((n, s) => n + s.seconds, 0)), 'total'],
		[`${days.filter((d) => d.minutes > 0).length} of 7`, 'days'],
		[String(new Set(week.map((s) => `${s.book}:${s.chapter}`)).size), 'chapters']
	]);

	function tip(i: number) {
		const { d, list, minutes } = days[i];
		const books = new Set(list.map((s) => s.book));
		let what: string;
		if (books.size === 1) {
			const cs = list.map((s) => s.chapter).sort((a, b) => a - b);
			const name = title(list[0]);
			const book = name.slice(0, name.lastIndexOf(' '));
			what = cs[0] !== cs.at(-1) ? `${book} ${cs[0]}–${cs.at(-1)}` : name;
		} else what = `${list.length} chapters`;
		return [`${weekday(d)} · ${minutes} min`, `${what} · ${list.length} ${list.length === 1 ? 'session' : 'sessions'}`];
	}
</script>

<PageHead title="History" subtitle="Everything you’ve read, newest first">
	{#snippet actions()}
		{#if ever.length}
			<Segmented
				layout="tight"
				label="Range"
				options={[
					{ value: 0, label: 'Today' },
					{ value: 1, label: 'Week' },
					{ value: 2, label: 'All' }
				]}
				bind:value={range}
			/>
			<Button
				kind="plain"
				label="Clear…"
				bind:el={clearButton}
				aria-haspopup="menu"
				aria-expanded={clearing}
				onclick={() => (clearing = !clearing)}
			/>
		{/if}
	{/snippet}
</PageHead>

{#if clearing}
	<Menu anchor={clearButton ?? null} align="end" width={200} onclose={() => (clearing = false)} label="Clear history">
		<MenuItem label="Clear Today" onclick={() => clear(today)} />
		<MenuItem label="Clear the Last 7 Days" onclick={() => clear(today - 6 * DAY)} />
		<MenuSeparator />
		<MenuItem label="Clear All History" onclick={() => clear(0)} />
	</Menu>
{/if}

{#if ever.length === 0}
	<PageEmpty
		glyph="history"
		title="Your reading will show up here"
		body="Each chapter you read is kept here — when, for how long and how far you got. It stays in this browser."
	/>
{:else}
	<div class="body">
		<ol class="timeline">
			{#each sessions as s, i (s.started)}
				{#if i === 0 || dayStart(s.started) !== dayStart(sessions[i - 1].started)}
					<li class="day">{dayLabel(s)}</li>
				{/if}
				<li>
					<button class="session" onclick={() => open(s)}>
						<span class="clock">{clock(s.started)}</span>
						<span class={['rail', { last: i === sessions.length - 1 }]}></span>
						<span class="main">
							<span class="top">
								<span class="title">{title(s)}</span>
								<Badge label={app.info(s.translation)?.abbrev ?? s.translation.toUpperCase()} />
								<span class="duration">{duration(s.seconds)}</span>
							</span>
							<span class="snippet"><CapsText text={snippet(s)} /></span>
							<span class="progress">
								<span class="track"><span class="fill" style:width="{progress(s) * 120}px"></span></span>
								<span class="status">
									{finished(s) ? 'Finished' : `${Math.round(progress(s) * 100)} % of chapter`}
								</span>
							</span>
						</span>
					</button>
				</li>
			{:else}
				<li class="none">Nothing read in this range.</li>
			{/each}
		</ol>

		<section class="week" aria-label="This week">
			<h2 class="week-title">This week</h2>
			<p class="week-sub">Minutes read per day</p>
			<div class="chart" role="img" aria-label="Minutes read per day">
				{#each days as day, i (day.d)}
					{@const isToday = day.d === today}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="col"
						onpointerenter={() => (hovered = day.minutes > 0 ? i : null)}
						onpointerleave={() => (hovered = null)}
					>
						{#if day.minutes === 0}
							<span class="nil"></span>
						{:else}
							<span
								class={['bar', { strong: isToday || hovered === i }]}
								style:height="{Math.max(4, (day.minutes / max) * 106)}px"
							>
								{#if isToday}<span class="bar-label">{day.minutes} min</span>{/if}
							</span>
						{/if}
						<span class={['weekday', { today: isToday }]}>{weekdayShort(day.d)}</span>
					</div>
				{/each}
				{#if hovered !== null}
					{@const [a, b] = tip(hovered)}
					<div
						class="tip"
						bind:offsetWidth={tipWidth}
						style:left="{Math.min(Math.max(hovered * 40 + 16 - tipWidth / 2, -12), 272 - tipWidth)}px"
					>
						<strong>{a}</strong>
						<span>{b}</span>
					</div>
				{/if}
			</div>
			<div class="stats">
				{#each stats as [value, label] (label)}
					<div class="stat">
						<span class="stat-value">{value}</span>
						<span class="stat-label">{label}</span>
					</div>
				{/each}
			</div>
		</section>
	</div>
{/if}

<style>
	.body {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 32px;
	}

	.timeline {
		flex: 0 1 584px;
		min-width: 0;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.day {
		height: 32px;
		padding: 4px 0 0 60px;
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.none {
		padding: 4px 0 16px 60px;
		color: var(--ink-3);
	}

	.session {
		position: relative;
		display: block;
		width: 100%;
		height: 84px;
		padding-left: 88px;
		text-align: left;
	}

	.clock {
		position: absolute;
		top: 0;
		right: calc(100% - 48px);
		color: var(--ink-3);
		font: 500 12px/20px var(--font-ui);
	}

	.rail::before {
		position: absolute;
		top: 6px;
		left: 63.5px;
		width: 9px;
		height: 9px;
		border-radius: 50%;
		background: var(--accent);
		content: '';
	}

	.rail:not(.last)::after {
		position: absolute;
		top: 19px;
		bottom: 0;
		left: 67.5px;
		width: 1px;
		background: var(--separator);
		content: '';
	}

	.main {
		display: block;
	}

	.top {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 20px;
	}

	.title {
		color: var(--ink);
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
	}

	.session:hover .title {
		color: var(--accent);
	}

	.duration {
		margin-left: auto;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.snippet {
		display: block;
		overflow: hidden;
		margin-top: 4px;
		padding-right: 8px;
		color: var(--ink-2);
		font: var(--type-snippet-sm);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.progress {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 16px;
		margin-top: 4px;
	}

	.track {
		position: relative;
		width: 120px;
		height: 4px;
		overflow: hidden;
		border-radius: 2px;
		background: var(--fill-strong);
	}

	.fill {
		position: absolute;
		inset: 0 auto 0 0;
		border-radius: 2px;
		background: var(--accent);
	}

	.status {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.week {
		flex: none;
		width: 320px;
		height: 326px;
		padding: 20px;
		border-radius: 16px;
		background: var(--sidebar);
		box-shadow: var(--shadow-panel);
	}

	.week-title {
		color: var(--ink);
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
	}

	.week-sub {
		margin-top: 2px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.chart {
		position: relative;
		display: flex;
		gap: 8px;
		height: 160px;
		margin-top: 16px;
	}

	.chart::after {
		position: absolute;
		top: 138px;
		right: 0;
		left: 0;
		height: 1px;
		background: var(--separator);
		content: '';
	}

	.col {
		position: relative;
		width: 32px;
	}

	.nil,
	.bar {
		position: absolute;
		bottom: 24px;
		left: 0;
		width: 32px;
	}

	.nil {
		height: 2px;
		border-radius: 1px;
		background: var(--fill-strong);
	}

	.bar {
		border-radius: 4px 4px 0 0;
		background: color-mix(in srgb, var(--accent) 55%, transparent);
	}

	.bar.strong {
		background: var(--accent);
	}

	.bar-label {
		position: absolute;
		bottom: calc(100% + 2px);
		left: 50%;
		color: var(--ink);
		font: 600 11px/16px var(--font-ui);
		transform: translateX(-50%);
		white-space: nowrap;
	}

	.weekday {
		position: absolute;
		top: 144px;
		right: -4px;
		left: -4px;
		color: var(--ink-3);
		font: 400 11px/16px var(--font-ui);
		text-align: center;
	}

	.weekday.today {
		color: var(--ink);
		font-weight: 600;
	}

	.tip {
		position: absolute;
		top: -6px;
		display: flex;
		flex-direction: column;
		padding: 6px 10px;
		border-radius: 8px;
		background: var(--raised);
		box-shadow: var(--shadow-popover);
		white-space: nowrap;
		pointer-events: none;
	}

	.tip strong {
		color: var(--ink);
		font: 600 12px/16px var(--font-ui);
	}

	.tip span {
		color: var(--ink-2);
		font: 400 11px/16px var(--font-ui);
	}

	.stats {
		display: flex;
		gap: 12px;
		margin-top: 16px;
		padding-top: 12px;
		box-shadow: inset 0 1px var(--separator);
	}

	/* Each widens by an equal share of what's spare, so they start where the Mac sets them. */
	.stat {
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
	}

	.stat-value {
		color: var(--ink);
		font: 400 20px/28px var(--font-reading);
	}

	.stat-label {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}
</style>
