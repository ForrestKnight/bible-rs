<!--
	Settings › Appearance (settings_ui.rs `appearance`): theme, accent colour, the toolbar's
	translation and the sidebar's style; names for the highlight colours; motion.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { HIGHLIGHT_COLORS } from '../core';
	import { ACCENTS, type Accent, type Theme } from '../settings';
	import { Icon, Keycap, Segmented, TextField, Toggle, type IconName } from '../ui';

	const app = getApp();
	const s = $derived(app.settings);

	const THEMES: { value: Theme | 'auto'; label: string; icon: IconName }[] = [
		{ value: 'light', label: 'Light', icon: 'sun' },
		{ value: 'sepia', label: 'Sepia', icon: 'page' },
		{ value: 'dark', label: 'Dark', icon: 'moon' },
		{ value: 'auto', label: 'Auto', icon: 'auto' }
	];
	const themeValue = $derived(s.themeSystem ? 'auto' : s.theme);

	function pickTheme(v: Theme | 'auto') {
		if (v === 'auto') {
			s.themeSystem = true;
		} else {
			s.themeSystem = false;
			app.dispatch({ type: 'setTheme', theme: v });
		}
	}

	const SWATCHES: Record<Accent, string> = {
		ink: '#2d4a9e',
		oxblood: '#8e2b2b',
		forest: '#2f6b4f',
		plum: '#6b3f7a',
		graphite: '#4a4d55'
	};
	const capital = (w: string) => w[0].toUpperCase() + w.slice(1);
</script>

<h3 class="group-title">Appearance</h3>
<div class="rows">
	<div class="row">
		<span class="row-label">Theme</span>
		<div class="segments" role="radiogroup" aria-label="Theme">
			{#each THEMES as t (t.value)}
				<button
					type="button"
					class={['segment', { on: themeValue === t.value }]}
					role="radio"
					aria-checked={themeValue === t.value}
					onclick={() => pickTheme(t.value)}
				>
					<Icon name={t.icon} size={16} heavy />{t.label}
				</button>
			{/each}
		</div>
	</div>
	<div class="row" style:height="64px">
		<span class="row-label">Accent colour<small>Tints selection, links and primary buttons</small></span>
		<div class="accents" role="radiogroup" aria-label="Accent colour">
			{#each ACCENTS as a (a)}
				<button
					type="button"
					class={['accent', { on: s.accent === a }]}
					role="radio"
					aria-checked={s.accent === a}
					onclick={() => s.accent !== a && app.dispatch({ type: 'setAccent', accent: a })}
				>
					<span class="dot" style:background={SWATCHES[a]}>
						{#if s.accent === a}<Icon name="check" size={12} heavy />{/if}
					</span>
					<span class="accent-label">{capital(a)}</span>
				</button>
			{/each}
		</div>
	</div>
	<div class="row">
		<span class="row-label">Show translation in toolbar</span>
		<Toggle bind:checked={s.toolbarTranslation} label="Show translation in toolbar" />
	</div>
	<div class="row">
		<span class="row-label">Sidebar</span>
		<Segmented
			options={[
				{ value: 'floating', label: 'Floating' },
				{ value: 'attached', label: 'Attached' }
			]}
			value={s.sidebarAttached ? 'attached' : 'floating'}
			onchange={(v) => (s.sidebarAttached = v === 'attached')}
			layout="tight"
			label="Sidebar"
		/>
	</div>
</div>

<h3 class="group-title">Highlight colours</h3>
<div class="rows">
	{#each HIGHLIGHT_COLORS as c, i (c)}
		<div class="row" style:height="48px">
			<span class="swatch" style:background="var(--swatch-{c})"></span>
			<span class="name">
				<TextField
					value={s.highlightNames[i] ?? ''}
					oninput={(e) => {
						const names = [...s.highlightNames];
						while (names.length < HIGHLIGHT_COLORS.length) names.push('');
						names[i] = e.currentTarget.value;
						s.highlightNames = names;
					}}
					placeholder={capital(c)}
					aria-label="Name for {c}"
				/>
			</span>
			<span class="key">Key</span>
			<Keycap label={String(i + 1)} />
		</div>
	{/each}
</div>
<p class="footnote">Names appear in the highlight menu and the Highlights filter.</p>

<h3 class="group-title">Motion</h3>
<div class="rows">
	<div class="row" style:height="48px">
		<span class="row-label">Reduce motion<small>Follows the system setting by default</small></span>
		<Toggle
			checked={s.reduceMotion}
			onchange={(on) => app.dispatch({ type: 'setReduceMotion', on })}
			label="Reduce motion"
		/>
	</div>
	<div class="row">
		<span class="row-label">Flash a verse after jumping to it</span>
		<Toggle bind:checked={s.flashJumps} label="Flash a verse after jumping to it" />
	</div>
</div>

<style>
	.segments {
		display: flex;
		gap: 2px;
		height: 32px;
		padding: 2px;
		border-radius: 16px;
		background: var(--fill);
	}

	.segment {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 28px;
		padding: 0 10px;
		border-radius: 14px;
		color: var(--ink-2);
		font: 500 13px/20px var(--font-ui);
	}

	.segment:hover:not(.on) {
		background: var(--fill);
	}

	.segment.on {
		background: var(--raised);
		box-shadow: var(--shadow-knob);
		color: var(--ink);
		font-weight: 600;
	}

	.accents {
		display: flex;
		gap: 4px;
		align-self: flex-start;
		margin-top: 10px;
	}

	.accent {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		width: 52px;
		height: 44px;
	}

	.dot {
		position: relative;
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		box-shadow: inset 0 0 0 0.5px rgb(255 255 255 / 0.2);
		color: #fff;
	}

	.accent:hover:not(.on) .dot {
		box-shadow:
			inset 0 0 0 0.5px rgb(255 255 255 / 0.2),
			0 0 0 3px var(--fill-strong);
	}

	.accent.on .dot {
		box-shadow:
			0 0 0 2px var(--surface),
			0 0 0 3.5px var(--ink);
	}

	.accent-label {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.accent.on .accent-label {
		color: var(--ink);
		font-weight: 600;
	}

	.swatch {
		flex: none;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.18);
	}

	.name {
		display: flex;
		width: 280px;
	}

	.name :global(> *) {
		flex: 1;
	}

	.key {
		margin-left: auto;
		color: var(--ink-3);
		font: var(--type-small);
	}
</style>
