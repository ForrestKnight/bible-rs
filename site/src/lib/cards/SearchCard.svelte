<script lang="ts">
	import Icon from '#lib/app/Icon.svelte';
	import SearchPanel from '#lib/app/SearchPanel.svelte';
	import Verses from '#lib/Verses.svelte';
	import { RESULTS } from '#lib/demo/script.js';
	import { easeOut, envelope, span } from '#lib/ease.js';
	import { loop } from '#lib/motion.svelte.js';
	import { JOHN_13 } from '#lib/scripture.js';
	import Fit from './Fit.svelte';

	const QUERY = 'love one another';
	const KEYS = [...QUERY].map((_, i) => 400 + i * 80);
	const RESULTS_AT = KEYS[KEYS.length - 1] + 200;
	const PICK = 2900;

	let { paused = false }: { paused?: boolean } = $props();

	const clock = loop(7600, 4200, () => paused);
	let t = $derived(clock.t);

	let query = $derived(QUERY.slice(0, KEYS.filter((k) => t >= k).length));
	let typing = $derived(KEYS.some((k) => t >= k && t < k + 450));
	let reveal = $derived(RESULTS.map((_, i) => easeOut(span(t, RESULTS_AT + i * 70, RESULTS_AT + i * 70 + 220))));
	let found = $derived(span(t, RESULTS_AT - 120, RESULTS_AT + 80));
	let opened = $derived(envelope(t, PICK, PICK + 300, 7000, 7400));
</script>

<div class="card-visual app" {@attach clock.observe}>
	<Fit width={506} height={304}>
		<div class="side"><SearchPanel {reveal} {found} selected={t >= PICK ? 0 : -1} /></div>
		<div class="main">
			<div class="field">
				<Icon name="search" size={11} />
				<span class="typed">{query}</span>
				<span class="caret" style:opacity={typing || Math.floor(t / 530) % 2 === 0 ? 1 : 0}></span>
				<span class="clear"><Icon name="clear" size={11} /></span>
			</div>
			<div class="reader" style:opacity={opened} style:transform="translateY({(1 - opened) * 8}px)">
				<div class="pill">Result 1 of 19<span>Esc to clear</span></div>
				<div class="heading"><span>JOHN</span><b>13</b></div>
				<div class="text"><Verses verses={JOHN_13.slice(3, 5)} tint={{ 34: 'var(--flash)', 35: 'var(--flash)' }} /></div>
			</div>
		</div>
	</Fit>
</div>

<style>
	.card-visual {
		background: var(--paper);
	}

	.side {
		position: absolute;
		left: 12px;
		top: 12px;
		width: 236px;
		height: 330px;
	}

	.main {
		position: absolute;
		left: 266px;
		top: 12px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		width: 220px;
	}

	.field {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 8px;
		border-radius: 13px;
		background: var(--field);
		box-shadow:
			0 0 0 1px var(--accent),
			0 0 0 3.5px color-mix(in srgb, var(--accent) 25%, transparent);
		color: var(--ink-2);
		font: 11px var(--sans);
	}

	.typed {
		color: var(--ink);
		white-space: pre;
	}

	.caret {
		width: 1px;
		height: 13px;
		margin-left: -6px;
		background: var(--accent);
	}

	.clear {
		display: grid;
		margin-left: auto;
		color: var(--ink-3);
	}

	.reader {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.pill {
		display: flex;
		align-self: center;
		gap: 8px;
		padding: 5px 10px;
		border-radius: 11px;
		background: var(--control);
		box-shadow: var(--control-edge);
		color: var(--ink-2);
		font: 9.5px/12px var(--sans);
	}

	.pill span {
		color: var(--ink-3);
	}

	.heading {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		color: var(--ink);
	}

	.heading span {
		color: var(--accent);
		font: 600 8px/10px var(--sans);
		letter-spacing: 0.14em;
	}

	.heading b {
		font: 300 30px/36px var(--serif);
	}

	.text {
		padding: 0 4px;
		color: var(--ink);
		font: 12px/20px var(--serif);
	}
</style>
