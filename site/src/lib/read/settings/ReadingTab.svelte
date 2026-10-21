<!--
	Settings › Reading (settings_ui.rs `reading`): a preview of John 1:1–2 as the reader sets it,
	the theme, the text's size, spacing and measure, and how verses are laid out.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { shortcutText } from '../keyboard';
	import { defaultTypography, type Theme } from '../settings';
	import { Button, Segmented, SelectField, Slider, Toggle } from '../ui';

	const app = getApp();
	const typo = $derived(app.settings.typo);

	/** Line length shown in characters; stored as the measure in ems (66 characters ≈ 32 em). */
	const CHARS_PER_EM = 66 / 32;

	const preview = $derived.by(() => {
		const id = app.settings.translation;
		const get = (verse: number) =>
			app.core
				?.verse(id, { book: 42, chapter: 1, verse })
				?.runs.filter((r) => !r.number)
				.map((r) => r.text)
				.join('') ?? '';
		return app.loaded.has(id) ? `${get(1)} ${get(2)}` : '';
	});

	const THEMES: { theme: Theme | null; label: string }[] = [
		{ theme: 'light', label: 'Light' },
		{ theme: 'sepia', label: 'Sepia' },
		{ theme: 'dark', label: 'Dark' },
		{ theme: null, label: 'Match System' }
	];

	const isOn = (t: Theme | null) => (t ? !app.settings.themeSystem && app.settings.theme === t : app.settings.themeSystem);

	function pickTheme(t: Theme | null) {
		if (t) {
			app.settings.themeSystem = false;
			app.dispatch({ type: 'setTheme', theme: t });
		} else {
			app.settings.themeSystem = true;
		}
	}

	let chars = $derived(Math.round(typo.measure * CHARS_PER_EM));
</script>

<div class="preview-card">
	<p class="preview-label">John 1</p>
	<p class={['preview-text', { justify: typo.justify }]}>
		{#if typo.verseNumbers}<span class="vn">1&nbsp;</span>{/if}{preview}
	</p>
</div>
<p class="preview-note">Preview updates as you change settings.</p>

<h3 class="group-title">Theme</h3>
<div class="themes" role="radiogroup" aria-label="Theme">
	{#each THEMES as t (t.label)}
		<button
			type="button"
			class={['theme', { on: isOn(t.theme) }]}
			role="radio"
			aria-checked={isOn(t.theme)}
			onclick={() => pickTheme(t.theme)}
		>
			{#if t.theme}
				<span class="card" data-theme-preview={t.theme}>
					<span class="strip"></span>
					<span class="sample">
						<span class="aa">Aa</span>
						<span class="bar" style:width="64px"></span>
						<span class="bar" style:width="72px"></span>
						<span class="bar" style:width="56px"></span>
					</span>
				</span>
			{:else}
				<span class="card system"><span data-theme-preview="light"></span><span data-theme-preview="dark"></span></span>
			{/if}
			<span class="theme-label">{t.label}</span>
		</button>
	{/each}
</div>

<h3 class="group-title">Text</h3>
<div class="rows">
	<div class="row">
		<span class="row-label">Font</span>
		<SelectField options={[{ value: 'literata', label: 'Literata' }]} value="literata" label="Font" width={220} />
	</div>
	<div class="row">
		<span class="row-label">Size</span>
		<span class="a small" aria-hidden="true">A</span>
		<span class="slider" style:width="200px">
			<Slider bind:value={typo.size} min={14} max={32} step={1} ticks={7} label="Size" valueText="{typo.size} points" />
		</span>
		<span class="a big" aria-hidden="true">A</span>
		<span class="value num">{typo.size} pt</span>
	</div>
	<div class="row">
		<span class="row-label">Line spacing</span>
		<span class="slider" style:width="256px">
			<Slider bind:value={typo.lineHeight} min={1.3} max={2} step={0.05} label="Line spacing" />
		</span>
		<span class="value num">{Math.round(typo.lineHeight * 100) / 100}</span>
	</div>
	<div class="row">
		<span class="row-label">Line length</span>
		<span class="slider" style:width="256px">
			<Slider
				bind:value={chars}
				min={45}
				max={90}
				step={1}
				label="Line length"
				valueText="{chars} characters"
				oninput={(v) => (typo.measure = v / CHARS_PER_EM)}
			/>
		</span>
		<span class="value num">{chars} ch</span>
	</div>
</div>

<h3 class="group-title">Layout</h3>
<div class="rows">
	<div class="row">
		<span class="row-label">Verses</span>
		<Segmented
			options={[
				{ value: 'paragraph', label: 'Paragraphs' },
				{ value: 'versePerLine', label: 'Verse per line' }
			]}
			bind:value={typo.mode}
			layout="fit"
			label="Verses"
		/>
	</div>
	<div class="row" style:height="48px">
		<span class="row-label">Words of Christ in red<small>{shortcutText('red', app.settings.shortcuts)}</small></span>
		<Toggle bind:checked={typo.redLetters} label="Words of Christ in red" />
	</div>
	<div class="row">
		<span class="row-label">Verse numbers</span>
		<Toggle bind:checked={typo.verseNumbers} label="Verse numbers" />
	</div>
	<div class="row" style:height="48px">
		<span class="row-label">Section headings<small>Where the translation has them (BSB, WEB)</small></span>
		<Toggle bind:checked={typo.headings} label="Section headings" />
	</div>
	<div class="row">
		<span class="row-label">Justify text</span>
		<Toggle bind:checked={typo.justify} label="Justify text" />
	</div>
</div>

<footer class="foot">
	<p>Literata and Inter are bundled under the SIL Open Font License.</p>
	<Button label="Reset to Defaults" onclick={() => (app.settings.typo = defaultTypography())} />
</footer>

<style>
	.preview-card {
		padding: 16px 24px 20px;
		border-radius: 12px;
		background: var(--paper);
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.preview-label {
		margin-bottom: 8px;
		color: var(--accent);
		font: 600 11px/16px var(--font-ui);
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.preview-text {
		display: -webkit-box;
		max-width: min(100%, var(--reading-measure));
		overflow: hidden;
		color: var(--ink);
		font-family: var(--font-reading);
		font-size: var(--reading-size);
		line-height: var(--lh);
		-webkit-line-clamp: 4;
		-webkit-box-orient: vertical;
		line-clamp: 4;
	}

	.preview-text.justify {
		text-align: justify;
	}

	.vn {
		font-family: var(--font-ui);
		font-weight: 600;
		font-size: round(calc(11 * var(--u)), 1px);
		line-height: calc(20 * var(--u));
		vertical-align: top;
		color: var(--verse);
	}

	.preview-note {
		margin-top: 8px;
		padding-left: 8px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.themes {
		display: flex;
		justify-content: space-between;
		height: 108px;
	}

	.theme {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		width: 136px;
	}

	.card {
		position: relative;
		display: flex;
		width: 136px;
		height: 84px;
		overflow: hidden;
		border-radius: 10px;
		background: var(--paper);
		box-shadow: 0 0 0 0.5px var(--border);
		transition: box-shadow var(--motion-fast);
	}

	.theme:hover:not(.on) .card {
		box-shadow:
			0 0 0 0.5px var(--border),
			0 0 0 3px var(--fill-strong);
	}

	.theme.on .card {
		box-shadow:
			0 0 0 2px var(--accent),
			0 0 0 5px var(--focus);
	}

	.strip {
		width: 28px;
		background: var(--sidebar);
	}

	.sample {
		display: flex;
		flex-direction: column;
		padding: 10px 0 0 12px;
		color: var(--ink);
		text-align: left;
	}

	.aa {
		height: 20px;
		margin-bottom: 6px;
		font: 400 15px/20px var(--font-reading);
	}

	.bar {
		height: 4px;
		margin-bottom: 6px;
		border-radius: 2px;
		background: color-mix(in srgb, var(--ink) 55%, transparent);
	}

	.system span {
		flex: 1;
		background: var(--paper);
	}

	.theme-label {
		color: var(--ink-2);
		font: var(--type-small);
	}

	.theme.on .theme-label {
		color: var(--accent);
		font-weight: 600;
	}

	.a {
		color: var(--ink-2);
		font-family: var(--font-reading);
		line-height: 20px;
	}

	.a.small {
		font-size: 13px;
	}

	.a.big {
		width: 24px;
		margin-left: -12px;
		font-size: 19px;
		text-align: right;
	}

	.slider {
		flex: none;
	}

	.num {
		width: 44px;
		text-align: right;
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 24px;
		color: var(--ink-3);
		font: var(--type-small);
	}
</style>
