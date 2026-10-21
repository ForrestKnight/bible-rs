<!--
	The toolbar's text & layout popover (settings_ui.rs `quick_text_settings`): the settings people
	change while reading.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { cmdText, shortcutText } from '../keyboard';
	import { THEMES } from '../settings';
	import { Checkbox, Popover } from '../ui';

	interface Props {
		anchor: HTMLElement;
		onclose: () => void;
		align?: 'start' | 'end';
	}

	let { anchor, onclose, align = 'start' }: Props = $props();

	const app = getApp();
	const s = $derived(app.settings);
	const NAMES = { light: 'Light', sepia: 'Sepia', dark: 'Dark' };
	const shortcut = (id: string) => {
		const t = shortcutText(id, s.shortcuts);
		return t ? ` (${t})` : '';
	};
</script>

<Popover {anchor} {onclose} {align} label="Text & layout" class="text-popover">
	<div class="body">
		<div class="themes" role="radiogroup" aria-label="Theme">
			{#each THEMES as t (t)}
				<button
					type="button"
					class="theme"
					class:on={!s.themeSystem && s.theme === t}
					data-theme-preview={t}
					role="radio"
					aria-checked={!s.themeSystem && s.theme === t}
					onclick={() => app.dispatch({ type: 'setTheme', theme: t })}
				>
					{NAMES[t]}
				</button>
			{/each}
		</div>
		<div class="size">
			<button type="button" class="step smaller" title="Smaller ({cmdText('−')})" onclick={() => app.dispatch({ type: 'fontSize', delta: -1 })}>
				A−
			</button>
			<span class="pt">{Math.round(s.typo.size)} pt</span>
			<button type="button" class="step bigger" title="Bigger ({cmdText('+')})" onclick={() => app.dispatch({ type: 'fontSize', delta: 1 })}>
				A+
			</button>
		</div>
		<div class="checks">
			<Checkbox
				label="Verse per line{shortcut('mode')}"
				checked={s.typo.mode === 'versePerLine'}
				onchange={() => app.dispatch({ type: 'toggleMode' })}
			/>
			<Checkbox
				label="Red letters{shortcut('red')}"
				checked={s.typo.redLetters}
				onchange={() => app.dispatch({ type: 'toggleRedLetters' })}
			/>
			<Checkbox label="Justify paragraphs" bind:checked={app.settings.typo.justify} />
		</div>
		<div class="separator"></div>
		<button
			type="button"
			class="more"
			onclick={() => {
				app.dispatch({ type: 'openSettings', tab: 'reading' });
				onclose();
			}}
		>
			More settings…
		</button>
	</div>
</Popover>

<style>
	.body {
		display: flex;
		flex-direction: column;
		min-width: 250px;
		padding: var(--menu-padding-y) var(--menu-padding-x);
	}

	.themes {
		display: flex;
		gap: 8px;
	}

	.theme {
		width: 72px;
		height: 44px;
		border-radius: 8px;
		background: var(--paper);
		box-shadow: inset 0 0 0 1px var(--separator);
		color: var(--ink);
		font: 400 13px/1 var(--font-reading);
	}

	.theme.on {
		box-shadow: inset 0 0 0 2px var(--accent);
	}

	.size {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 6px;
		padding: 0 2px;
	}

	.step {
		height: 24px;
		padding: 0 8px;
		border-radius: 8px;
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--ink);
		font: 500 13px/1 var(--font-ui);
	}

	.bigger {
		font-size: 15px;
	}

	.step:hover {
		background: var(--fill);
		box-shadow: inset 0 0 0 1px var(--ink-3);
	}

	.step:active {
		background: var(--fill-strong);
	}

	.pt {
		min-width: 40px;
		text-align: center;
		font: var(--type-ui);
		font-variant-numeric: tabular-nums;
	}

	.checks {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-top: 10px;
		padding: 0 2px;
	}

	.separator {
		height: 1px;
		margin: 9px 0 4px;
		background: var(--separator);
	}

	.more {
		height: var(--menu-item-height);
		padding: 0 var(--menu-item-padding);
		margin: 0 calc(-1 * var(--menu-item-padding) + 2px);
		border-radius: var(--menu-item-radius);
		text-align: left;
		font: var(--type-ui);
		color: var(--ink);
	}

	.more:hover,
	.more:focus-visible {
		background: var(--fill);
		outline: none;
	}
</style>
