<!--
	First visit (onboarding.rs): a welcome with a choice of translation, then the theme and reading
	size; the reader opens after. It takes the whole page in the light palette (dark when the system
	is), and the theme cards preview the others.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { getApp } from '../app.svelte';
	import { shortcutText } from '../keyboard';
	import type { Theme } from '../settings';
	import { AppIcon, Button, CapsText, Checkbox, Icon, Radio, Slider } from '../ui';

	interface Samples {
		john: { id: string; abbrev: string; name: string; year: number | null; verse: string }[];
		psalm: { heading: string; text: string };
	}

	const STEPS = ['Translation', 'Theme', 'Ready'];
	const THEMES: [Theme, string][] = [
		['light', 'Light'],
		['sepia', 'Sepia'],
		['dark', 'Dark']
	];
	/** Literata's baseline in its line box, as a fraction of the size past half the line. */
	const LITERATA_BASELINE = (1.177 - 0.308) / 2;

	const app = getApp();
	const s = $derived(app.settings);
	const dark = new MediaQuery('prefers-color-scheme: dark');
	let step = $state(app.demo.onboarding === '2' ? 1 : 0);
	let samples = $state<Samples | null>(null);

	onMount(() => {
		void fetch('/read/onboarding.json')
			.then((r) => (r.ok ? r.json() : null))
			.then((d: Samples | null) => (samples = d));
	});

	function finish() {
		s.onboarded = true;
		app.onboarding = false;
	}

	const next = () => (step === 0 ? (step = 1) : finish());

	function keydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement)) {
			e.preventDefault();
			next();
		} else if (e.key === 'Escape' && step === 1) {
			e.preventDefault();
			step = 0;
		}
	}

	// The preview's type, at the reading size (onboarding.rs `preview`).
	const k = $derived(s.typo.size / 20);
	const titleSize = $derived(Math.round(17 * k));
	const titleLine = $derived(Math.round(28 * k));
	const small = $derived(Math.round(13 * k));
	const caps = $derived(/(?<!\p{L})\p{Lu}{2,}(?!\p{L})/u.test(samples?.psalm.heading ?? ''));
	const bodyTop = $derived(38 + titleLine + (caps ? LITERATA_BASELINE * (titleSize - small) : 0) + 6);
	const bodySize = $derived(Math.round(15 * k));
	const pitch = $derived(Math.round(24 * k) + 2);
	const box = $derived(Math.round(20 * k));
	const rows = $derived(Math.max(1, Math.floor((288 - 24 - bodyTop + 2) / pitch)));
</script>

<svelte:window onkeydown={keydown} />

<div class="onboarding" data-theme-preview={dark.current ? 'dark' : 'light'} role="dialog" aria-modal="true" aria-label="Welcome to Bible">
	<ol class="steps" aria-label="Steps">
		{#each STEPS as name, i (name)}
			{#if i > 0}<li class="rule" aria-hidden="true"></li>{/if}
			<li class={['step', { current: i === step, done: i < step }]} aria-current={i === step ? 'step' : undefined}>
				<span class="dot">{#if i < step}<Icon name="check" size={10} heavy />{:else}{i + 1}{/if}</span>
				{name}
			</li>
		{/each}
	</ol>

	<div class="stage">
		{#if step === 0}
			<div class="welcome">
				<AppIcon size={64} />
				<h1 class="display">Welcome to Bible</h1>
				<p class="lede">Just the Bible — fast, beautifully typeset, and kept in this browser.</p>
				<div class="choose">
					<h2>Choose a translation to start with</h2>
					<span>Public domain · works offline · more in Settings</span>
				</div>
				<div class="cards" role="radiogroup" aria-label="Translation">
					{#each samples?.john ?? [] as t (t.id)}
						{@const on = s.translation === t.id}
						<button
							class={['card', { on }]}
							role="radio"
							aria-checked={on}
							onclick={() => app.dispatch({ type: 'setTranslation', id: t.id })}
						>
							<span class="card-top">
								<span class="abbrev">{t.abbrev}</span>
								{#if t.year}<span class="year">{t.year}</span>{/if}
								<span class="check">{#if on}<Icon name="check" size={10} heavy />{/if}</span>
							</span>
							<span class="name">{t.name}</span>
							<span class="verse">{t.verse}</span>
						</button>
					{/each}
				</div>
			</div>
		{:else}
			<div class="appearance">
				<h1 class="display small">How should it read?</h1>
				<p class="lede">You can switch any time with {shortcutText('theme', s.shortcuts)}.</p>
				<div class="themes" role="radiogroup" aria-label="Theme">
					{#each THEMES as [theme, label] (theme)}
						{@const on = s.theme === theme && !s.themeSystem}
						<div class="theme">
							<button
								class={['preview', { on }]}
								data-theme-preview={theme}
								role="radio"
								aria-checked={on}
								aria-label={label}
								onclick={() => {
									s.themeSystem = false;
									app.dispatch({ type: 'setTheme', theme });
								}}
								style:--title-size="{titleSize}px"
								style:--title-line="{titleLine}px"
								style:--body-top="{bodyTop}px"
								style:--body-size="{bodySize}px"
								style:--pitch="{pitch}px"
								style:--box="{box}px"
								style:--rows={rows}
							>
								<span class="kicker">Psalm 23</span>
								{#if samples}
									{#if samples.psalm.heading}<span class="psalm-title"><CapsText text={samples.psalm.heading} /></span>{/if}
									<span class="psalm"><CapsText text={samples.psalm.text} /></span>
								{/if}
							</button>
							<div class={['pick', { on }]}>
								<Radio
									name="onboarding-theme"
									checked={on}
									{label}
									onchange={() => {
										s.themeSystem = false;
										app.dispatch({ type: 'setTheme', theme });
									}}
								/>
							</div>
						</div>
					{/each}
				</div>
				<div class="size">
					<span class="size-label">Text size</span>
					<span class="a" aria-hidden="true">A</span>
					<div class="slider">
						<Slider bind:value={s.typo.size} min={14} max={32} step={1} ticks={7} label="Text size" valueText="{s.typo.size} points" />
					</div>
					<span class="a big" aria-hidden="true">A</span>
					<span class="value">{Math.round(s.typo.size)} pt</span>
					<span class="sep" aria-hidden="true"></span>
					<Checkbox bind:checked={s.themeSystem} label="Match system appearance" />
				</div>
			</div>
		{/if}
	</div>

	<footer class="footer">
		{#if step === 0}
			<p class="hint">
				<Icon name="download" size={16} heavy />
				Have a licensed translation (NASB, ESV, NLT…)? You can import it later in Settings.
			</p>
		{:else}
			<Button kind="plain" label="Back" onclick={() => (step = 0)} />
		{/if}
		<Button kind="primary" label="Continue" icon="arrow-right" trailing onclick={next} />
	</footer>
</div>

<style>
	.onboarding {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		flex-direction: column;
		background: var(--paper);
		color: var(--ink);
		font: var(--type-ui);
	}

	.steps {
		position: absolute;
		top: 16px;
		left: 50%;
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 0;
		padding: 0;
		list-style: none;
		transform: translateX(-50%);
		white-space: nowrap;
	}

	.step {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ink-3);
		font: 500 12px/16px var(--font-ui);
	}

	.step.current {
		color: var(--ink);
		font-weight: 600;
	}

	.dot {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: var(--fill);
		color: var(--ink-3);
		font: 600 11px/16px var(--font-ui);
	}

	.current .dot,
	.done .dot {
		background: var(--accent);
		color: var(--on-accent);
	}

	.rule {
		width: 32px;
		height: 1px;
		background: var(--separator);
	}

	.stage {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}

	.welcome,
	.appearance {
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	.welcome {
		padding-top: 68px;
	}

	.appearance {
		padding-top: 76px;
	}

	.display {
		margin-top: 12px;
		color: var(--ink);
		font: 300 40px/48px var(--font-reading);
		font-variation-settings: 'opsz' 48;
		letter-spacing: -0.015em;
	}

	.display.small {
		margin-top: 0;
		font-size: 32px;
		line-height: 40px;
	}

	.lede {
		margin-top: 12px;
		color: var(--ink-2);
		font: 400 15px/24px var(--font-ui);
	}

	.appearance .lede {
		margin-top: 6px;
	}

	.choose {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		width: 832px;
		margin-top: 24px;
	}

	.choose h2 {
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.choose span {
		color: var(--ink-3);
		font: var(--type-small);
	}

	.cards {
		display: grid;
		grid-template-columns: repeat(3, 266px);
		gap: 16px;
		margin-top: 24px;
	}

	.card {
		display: flex;
		flex-direction: column;
		height: 114px;
		padding: 14px;
		border-radius: 12px;
		background: var(--surface);
		box-shadow:
			0 0 0 0.5px var(--border),
			0 1px 4px rgb(0 0 0 / 0.04);
		text-align: left;
	}

	.card:hover {
		box-shadow:
			0 0 0 0.5px var(--ink-3),
			0 1px 4px rgb(0 0 0 / 0.04);
	}

	.card.on {
		background: var(--accent-soft);
		box-shadow: 0 0 0 2px var(--accent);
	}

	.card-top {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 18px;
	}

	.abbrev {
		color: var(--ink);
		font: 700 13px/16px var(--font-ui);
	}

	.on .abbrev {
		color: var(--accent);
	}

	.year {
		flex: 1;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.check {
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		margin-left: auto;
		border-radius: 50%;
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--on-accent);
	}

	.on .check {
		background: var(--accent);
		box-shadow: none;
	}

	.name {
		margin-top: 4px;
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.verse {
		display: -webkit-box;
		margin-top: 8px;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		color: var(--ink);
		font: 400 13px/20px var(--font-reading);
	}

	.themes {
		display: flex;
		gap: 24px;
		margin-top: 24px;
	}

	.theme {
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	.preview {
		position: relative;
		width: 244px;
		height: 288px;
		overflow: hidden;
		border-radius: 14px;
		background: var(--paper);
		box-shadow:
			0 0 0 0.5px var(--border),
			0 4px 32px rgb(0 0 0 / 0.06);
		color: var(--ink);
		text-align: left;
	}

	.preview:hover {
		box-shadow:
			0 0 0 0.5px var(--ink-3),
			0 4px 32px rgb(0 0 0 / 0.06);
	}

	.preview.on {
		box-shadow:
			0 0 0 2px var(--accent),
			0 0 0 6px var(--focus);
	}

	.kicker {
		position: absolute;
		top: 24px;
		left: 24px;
		color: var(--accent);
		font: 600 9px/12px var(--font-ui);
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.psalm-title {
		position: absolute;
		top: 38px;
		right: 0;
		left: 24px;
		overflow: hidden;
		font: 400 var(--title-size) / var(--title-line) var(--font-reading);
		white-space: nowrap;
	}

	/* Rows `pitch` apart, each set in a `box`-tall line as the Mac does. */
	.psalm {
		position: absolute;
		top: calc(var(--body-top) - (var(--pitch) - var(--box)) / 2);
		right: 24px;
		left: 24px;
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: var(--rows);
		line-clamp: var(--rows);
		overflow: hidden;
		font: 400 var(--body-size) / var(--pitch) var(--font-reading);
		white-space: pre-line;
	}

	.pick {
		display: flex;
		justify-content: center;
		height: 16px;
		margin-top: 14px;
	}

	.pick :global(.radio) {
		font: 500 13px/16px var(--font-ui);
	}

	.pick.on :global(.radio) {
		color: var(--accent);
		font-weight: 600;
	}

	.size {
		display: flex;
		align-items: center;
		height: 24px;
		margin-top: 32px;
	}

	.size-label {
		margin-right: 16px;
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.a {
		color: var(--ink-2);
		font: 400 14px/24px var(--font-reading);
	}

	.a.big {
		font-size: 22px;
	}

	.slider {
		width: 280px;
		margin: 0 12px;
	}

	.value {
		width: 44px;
		margin-left: 16px;
		color: var(--ink-2);
		font: 500 13px/16px var(--font-ui);
	}

	.sep {
		width: 1px;
		height: 20px;
		margin: 0 16px;
		background: var(--separator);
	}

	.footer {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: space-between;
		height: 72px;
		padding: 0 32px;
		box-shadow: inset 0 1px var(--separator);
	}

	.hint {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ink-2);
		font: var(--type-small);
	}
</style>
