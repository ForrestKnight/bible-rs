<!--
	Reading Plans (pages/plans.rs): the plans under way, each a card with its progress and a month
	of days, then the catalogue to start from; Browse and Completed list the rest. A plan's card
	opens its day (PlanDay).
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Badge, Button, EmptyState, Icon, PillTab } from '../ui';
	import PageHead from './PageHead.svelte';
	import PlanDay from './PlanDay.svelte';
	import { DAY, ICONS, MONTHS, Under, minutes, openPassage, passageSize, plan, plans, plansPage } from './plans.svelte';

	const app = getApp();

	const started = $derived(app.plans());
	const active = $derived(started.filter((s) => s.finished === null));
	const completed = $derived(started.filter((s) => s.finished !== null));
	const q = $derived(app.pageQuery.trim().toLowerCase());
	const matches = (p: { title: string; description: string }) =>
		!q || p.title.toLowerCase().includes(q) || p.description.toLowerCase().includes(q);
	const browse = $derived(plans().filter((p) => !active.some((s) => s.id === p.id) && matches(p)));
	const done = $derived(
		completed.flatMap((s) => {
			const p = plan(s.id);
			return p && matches(p) ? [p] : [];
		})
	);
	const unders = $derived(
		active.flatMap((s) => {
			const p = plan(s.id);
			return p && matches(p) ? [new Under(p, s, app)] : [];
		})
	);

	const opened = $derived.by(() => {
		const o = plansPage.open;
		const p = o && plan(o.id);
		const s = o && started.find((x) => x.id === o.id);
		return o && p && s ? { under: new Under(p, s, app), day: o.day } : null;
	});

	// The demo's `plan-day:<id>@<day>` opens a day.
	$effect(() => {
		const [id, day] = (app.demo['plan-day'] ?? '').split('@');
		if (id && started.some((s) => s.id === id)) plansPage.open = { id, day: Number(day) || 1 };
	});

	$effect(() => {
		void app.ensure(app.textTranslation());
	});

	function start(id: string) {
		app.dispatch({ type: 'startPlan', id });
		plansPage.tab = 'active';
	}

	const dayLabel = (u: Under, d: number) => u.days[d - 1]?.label ?? '';
	const date = (unix: number) => new Date(unix * 1000);
</script>

{#if opened}
	<PlanDay under={opened.under} day={opened.day} />
{:else}
	<PageHead title="Reading Plans" subtitle="Read a little every day">
		{#snippet actions()}
			<div class="tabs" role="tablist" aria-label="Plans">
				<PillTab
					label="Active"
					count={active.length}
					selected={plansPage.tab === 'active'}
					onclick={() => (plansPage.tab = 'active')}
				/>
				<PillTab
					label="Browse"
					count={browse.length}
					selected={plansPage.tab === 'browse'}
					onclick={() => (plansPage.tab = 'browse')}
				/>
				<PillTab
					label="Completed"
					count={completed.length}
					selected={plansPage.tab === 'completed'}
					onclick={() => (plansPage.tab = 'completed')}
				/>
			</div>
		{/snippet}
	</PageHead>

	{#snippet grid(list: typeof browse, action: string)}
		<div class="grid">
			{#each list as p (p.id)}
				<div class="card">
					<span class="tile"><Icon name={ICONS[p.icon]} size={20} /></span>
					<Button size="sm" label={action} class="start" onclick={() => start(p.id)} />
					<h3 class="card-title">{p.title}</h3>
					<p class="meta">{p.meta}</p>
					<p class="description">{p.description}</p>
				</div>
			{/each}
		</div>
	{/snippet}

	{#if plansPage.tab === 'active'}
		{#each unders as u (u.info.id)}
			{@const len = u.len}
			{@const day = Math.min(u.today, len)}
			{@const current = u.currentDay()}
			{@const doneDays = u.doneDays()}
			{@const streak = u.streak()}
			{@const words = u.day(Math.max(day, 1)).reduce((n, p) => n + passageSize(app, p)[1], 0)}
			{@const s = date(u.state.started)}
			{@const block = Math.floor((Math.max(day, 1) - 1) / 30)}
			{@const first = block * 30 + 1}
			{@const last = Math.min(first + 29, len)}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div
				class="hero"
				role="link"
				tabindex="-1"
				onclick={() => (plansPage.open = { id: u.info.id, day: current })}
			>
				<div class="hero-main">
					<Badge kind="accent" label={u.today > len ? 'Last day passed' : `Day ${day} of ${len}`} />
					<h2 class="hero-title">{u.info.title}</h2>
					<p class="hero-description">
						{u.info.description} Started {s.getDate()}
						{MONTHS[s.getMonth()]}.
					</p>
					<div class="hero-progress">
						<span class="track"><span class="fill" style:width="{(320 * doneDays) / len}px"></span></span>
						<span>{doneDays} of {len} days</span>
					</div>
					<div class="hero-actions">
						<Button
							kind="primary"
							size="lg"
							icon="book"
							label="Continue · {dayLabel(u, current)}"
							onclick={(e) => {
								e.stopPropagation();
								const p = u.next(current);
								if (p) openPassage(app, p);
							}}
						/>
						{#if streak > 0}
							<span class="streak"><Icon name="flame" size={16} heavy />{streak}-day streak</span>
						{/if}
						{#if words > 0}<span class="today">~{minutes(words)} min today</span>{/if}
					</div>
				</div>
				<div class="calendar">
					<h3 class="month">{MONTHS[date(u.state.started + (first - 1) * DAY).getMonth()]}</h3>
					<div class="dots">
						{#each { length: last - first + 1 }, k (k)}
							{@const d = first + k}
							<span
								class={[
									'dot',
									d === u.today ? 'today' : u.dayDone(d) ? 'read' : d < u.today ? 'missed' : 'future'
								]}
								title="Day {d}"
							></span>
						{/each}
					</div>
					<div class="legend">
						<span><i class="read"></i>Read</span>
						<span><i class="missed"></i>Missed</span>
						<span><i class="today"></i>Today</span>
					</div>
				</div>
			</div>
		{/each}
		{#if active.length === 0}
			<p class="nothing">No plan under way — start one below.</p>
		{/if}
		{#if browse.length}
			<div class="section">
				<h2>{active.length ? 'Start another plan' : 'Start a plan'}</h2>
				<span>{browse.length} {browse.length === 1 ? 'plan' : 'plans'}</span>
			</div>
			{@render grid(browse, 'Start')}
		{/if}
	{:else if plansPage.tab === 'browse'}
		{@render grid(browse, 'Start')}
	{:else if done.length}
		{@render grid(done, 'Restart')}
	{:else}
		<div class="empty">
			<EmptyState
				glyph="checklist"
				tone="accent"
				title={completed.length ? 'No finished plans match' : 'No finished plans yet'}
				body={completed.length
					? 'Try other words in the search field.'
					: 'Plans you finish show up here, ready to read again.'}
				width={360}
				minTop={102}
			>
				{#snippet buttons()}
					<Button label="Browse plans" onclick={() => (plansPage.tab = 'browse')} />
				{/snippet}
			</EmptyState>
		</div>
	{/if}
{/if}

<style>
	.tabs {
		display: flex;
		gap: 4px;
	}

	.hero {
		position: relative;
		display: flex;
		align-items: center;
		gap: 32px;
		height: 232px;
		margin-bottom: 20px;
		padding: 28px;
		border-radius: 20px;
		background: var(--sidebar);
		cursor: default;
	}

	.hero-main {
		flex: 1;
		align-self: flex-start;
		min-width: 0;
	}

	.hero-title {
		margin-top: 6px;
		color: var(--ink);
		font: var(--type-heading);
		font-variation-settings: 'opsz' 28;
		letter-spacing: var(--type-heading-tracking);
	}

	.hero-description {
		overflow: hidden;
		margin-top: 6px;
		color: var(--ink-2);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.hero-progress {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 16px;
		margin-top: 14px;
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.track {
		position: relative;
		flex: none;
		width: 320px;
		height: 6px;
		overflow: hidden;
		border-radius: 3px;
		background: var(--fill-strong);
	}

	.fill {
		position: absolute;
		inset: 0 auto 0 0;
		border-radius: 3px;
		background: var(--accent);
	}

	.hero-actions {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-top: 18px;
	}

	.streak {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--ink);
		font: 600 13px/16px var(--font-ui);
	}

	.streak :global(svg) {
		color: var(--swatch-orange);
	}

	.today {
		color: var(--ink-2);
		font: 400 13px/16px var(--font-ui);
	}

	.calendar {
		flex: none;
		width: 334px;
		height: 126px;
		padding: 20px;
		border-radius: 14px;
		background: var(--paper);
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.month {
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.dots {
		display: grid;
		grid-template-columns: repeat(15, 14px);
		gap: 6px;
		height: 34px;
		margin-top: 10px;
	}

	.dot {
		width: 14px;
		height: 14px;
		border-radius: 50%;
	}

	.dot.future {
		background: var(--fill-strong);
	}

	.dot.read {
		background: var(--accent);
	}

	.dot.missed {
		box-shadow: inset 0 0 0 1.5px var(--ink-3);
	}

	.dot.today {
		background: var(--raised);
		box-shadow: inset 0 0 0 2px var(--accent);
	}

	.legend {
		display: flex;
		gap: 12px;
		margin-top: 10px;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.legend span {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}

	.legend i {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.legend .read {
		background: var(--accent);
	}

	.legend .missed {
		box-shadow: inset 0 0 0 1.5px var(--ink-3);
	}

	.legend .today {
		box-shadow: inset 0 0 0 2px var(--accent);
	}

	.nothing {
		height: 40px;
		color: var(--ink-2);
	}

	.section {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		height: 40px;
	}

	.section h2 {
		color: var(--ink);
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
	}

	.section span {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, 300px);
		gap: 16px;
	}

	.card {
		position: relative;
		padding: 16px 16px 16px;
		border-radius: 14px;
		background: var(--surface);
		box-shadow:
			0 0 0 0.5px var(--border),
			0 1px 4px rgb(0 0 0 / 0.04);
	}

	.card:hover {
		box-shadow: var(--shadow-popover);
	}

	.tile {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent);
	}

	.card :global(.start) {
		position: absolute;
		top: 24px;
		right: 16px;
	}

	.card-title {
		margin-top: 4px;
		color: var(--ink);
		font: 600 15px/20px var(--font-ui);
	}

	.meta {
		margin-top: 4px;
		color: var(--ink-3);
		font: 500 12px/16px var(--font-ui);
	}

	/* The Mac's wrapped text keeps a line's leading below its glyphs, not split around them. */
	.description {
		position: relative;
		top: -2px;
		margin-top: 4px;
		color: var(--ink-2);
	}

	.empty {
		position: absolute;
		top: var(--toolbar);
		right: 40px;
		bottom: 0;
		left: 40px;
	}
</style>
