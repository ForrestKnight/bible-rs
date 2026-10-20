<script lang="ts">
	import Verses from '#lib/Verses.svelte';
	import { loop } from '#lib/motion.svelte.js';
	import { JOHN_1 } from '#lib/scripture.js';
	import Fit from './Fit.svelte';

	/** The app's three themes (crates/app/assets/design-tokens.json). */
	const THEMES = [
		{ name: 'Light', paper: '#ffffff', ink: '#1c1d21', ink2: '#6e6f76', verse: '#7383b0', accent: '#2d4a9e', hl: '#fbe7a1', control: '#ffffff', edge: 'rgba(0, 0, 0, 0.12)', separator: '#e4e4e8' },
		{ name: 'Sepia', paper: '#f6eedc', ink: '#3a2e22', ink2: '#73624e', verse: '#8c7a62', accent: '#3c5490', hl: '#eed896', control: '#fbf5e8', edge: 'rgba(58, 46, 34, 0.16)', separator: '#e0d2b4' },
		{ name: 'Dark', paper: '#161618', ink: '#e6e3dc', ink2: '#9c9ca3', verse: '#8592bc', accent: '#8ea6f2', hl: '#584a1e', control: '#303034', edge: 'rgba(255, 255, 255, 0.1)', separator: '#2e2e33' }
	];
	const STEP = 1500;

	let { paused = false }: { paused?: boolean } = $props();

	const clock = loop(STEP * THEMES.length, STEP, () => paused);
	let active = $derived(Math.floor(clock.t / STEP));
</script>

<div class="card-visual" {@attach clock.observe}>
	<Fit width={506} height={304}>
		<div class="row">
			{#each THEMES as theme, i (theme.name)}
				<div class={['theme', { active: i === active }]}>
					<div
						class="tile"
						style:--paper={theme.paper}
						style:--ink={theme.ink}
						style:--ink-2={theme.ink2}
						style:--verse={theme.verse}
						style:--accent={theme.accent}
						style:--control={theme.control}
						style:--edge={theme.edge}
						style:--separator={theme.separator}
					>
						<div class="bar">
							<span class="lights"><i></i><i></i><i></i></span>
							<span class="capsule">John 1<i></i>KJV</span>
						</div>
						<div class="head">
							<b>John</b>
							<span>The Gospel According to St. John</span>
							<small>CHAPTER 1</small>
						</div>
						<div class="text"><Verses verses={JOHN_1} tint={{ 1: theme.hl }} /></div>
					</div>
					<span class="name">{theme.name}</span>
				</div>
			{/each}
		</div>
	</Fit>
</div>

<style>
	.card-visual {
		background: radial-gradient(ellipse 70% 60% at 50% 0%, rgba(110, 140, 255, 0.08), transparent);
	}

	.row {
		position: absolute;
		inset: 34px 0 0;
		display: flex;
		justify-content: center;
		gap: 16px;
	}

	.theme {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
	}

	.tile {
		display: flex;
		flex-direction: column;
		width: 146px;
		height: 200px;
		overflow: hidden;
		border-radius: 9px;
		background: var(--paper);
		box-shadow: 0 0 0 1px var(--line);
		color: var(--ink);
		transition:
			transform 0.4s cubic-bezier(0.3, 1.4, 0.5, 1),
			box-shadow 0.4s;
	}

	.active .tile {
		box-shadow:
			0 0 0 1.5px color-mix(in srgb, var(--fg) 90%, transparent),
			0 0 0 5px color-mix(in srgb, var(--fg) 10%, transparent),
			0 14px 30px -6px rgba(0, 0, 0, 0.45);
		transform: translateY(-6px);
	}

	.bar {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 22px;
		flex-shrink: 0;
	}

	.lights {
		position: absolute;
		left: 7px;
		display: flex;
		gap: 3px;
	}

	.lights i {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: #ff5f57;
	}

	.lights i:nth-child(2) {
		background: #febc2e;
	}

	.lights i:nth-child(3) {
		background: #28c840;
	}

	.capsule {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 12px;
		padding: 0 6px;
		border-radius: 6px;
		background: var(--control);
		box-shadow:
			0 0 0 0.5px var(--edge),
			0 1px 3px rgba(0, 0, 0, 0.08);
		font: 600 6px/8px var(--sans);
	}

	.capsule i {
		width: 0.5px;
		height: 6px;
		background: var(--separator);
	}

	.head {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding-top: 8px;
	}

	.head b {
		font: 300 17px/20px var(--serif);
	}

	.head span {
		color: var(--ink-2);
		font: italic 5.5px/8px var(--serif);
	}

	.head small {
		padding-top: 3px;
		color: var(--accent);
		font: 600 4.5px/8px var(--sans);
		letter-spacing: 0.12em;
	}

	.text {
		padding: 8px 14px 0;
		font: 8.2px/13.5px var(--serif);
	}

	.name {
		color: var(--muted);
		font-size: 12px;
		line-height: 14px;
		transition: color 0.3s;
	}

	.active .name {
		color: var(--fg);
	}
</style>
