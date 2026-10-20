<script lang="ts">
	import VerseMenu from '#lib/app/VerseMenu.svelte';
	import Verses from '#lib/Verses.svelte';
	import { envelope, span } from '#lib/ease.js';
	import { loop } from '#lib/motion.svelte.js';
	import { ROMANS_8_28 } from '#lib/scripture.js';
	import Fit from './Fit.svelte';

	const SELECT = 400;
	const MENU = 700;
	const PICK = 1900;

	let { paused = false }: { paused?: boolean } = $props();

	const clock = loop(7000, 2600, () => paused);
	let t = $derived(clock.t);

	let menu = $derived(envelope(t, MENU, MENU + 180, 6200, 6450));
	let selected = $derived(span(t, SELECT, SELECT + 150) * (1 - span(t, 6500, 6900)));
	let picked = $derived(span(t, PICK, PICK + 260) * (1 - span(t, 6500, 6900)));
	let tint = $derived(
		picked > 0
			? `color-mix(in srgb, var(--hl-yellow) ${picked * 100}%, var(--accent-soft))`
			: `color-mix(in srgb, var(--accent-soft) ${selected * 100}%, transparent)`
	);
</script>

<div class="card-visual app" {@attach clock.observe}>
	<Fit width={506} height={304}>
		<div class="page">
			<div class="label">ROMANS 8</div>
			<Verses verses={ROMANS_8_28} tint={{ 28: tint }} />
		</div>
		<div class="menu" style:opacity={menu} style:transform="translateY({(1 - menu) * -6}px) scale({0.97 + menu * 0.03})">
			<VerseMenu picked={t >= PICK && t < 6500 ? 0 : -1} />
		</div>
		<div class="fade"></div>
	</Fit>
</div>

<style>
	.card-visual {
		background: var(--paper);
	}

	.page {
		position: absolute;
		left: 30px;
		top: 22px;
		width: 248px;
		color: var(--ink);
		font: 13px/22px var(--serif);
	}

	.label {
		padding-bottom: 10px;
		color: var(--accent);
		font: 600 8px/10px var(--sans);
		letter-spacing: 0.14em;
	}

	.menu {
		position: absolute;
		left: 290px;
		top: 26px;
		transform-origin: 0 0;
	}

	.fade {
		position: absolute;
		inset: auto 0 0;
		height: 40px;
		background: linear-gradient(transparent, var(--paper));
	}
</style>
