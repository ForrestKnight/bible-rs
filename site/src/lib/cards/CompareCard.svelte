<script lang="ts">
	import ComparePopover from '#lib/app/ComparePopover.svelte';
	import { easeOut, envelope, span } from '#lib/ease.js';
	import { loop } from '#lib/motion.svelte.js';
	import Fit from './Fit.svelte';

	const ROWS = 400;
	const STEP = 140;

	let { paused = false }: { paused?: boolean } = $props();

	const clock = loop(6400, 2400, () => paused);
	let t = $derived(clock.t);

	let shown = $derived(envelope(t, 0, 250, 5700, 6200));
	let reveal = $derived([0, 1, 2, 3, 4].map((i) => easeOut(span(t, ROWS + i * STEP, ROWS + i * STEP + 260))));
</script>

<div class="card-visual app" {@attach clock.observe}>
	<Fit width={506} height={304}>
		<div class="popover" style:opacity={shown} style:transform="translateY({(1 - shown) * 8}px)">
			<ComparePopover {reveal} />
		</div>
		<div class="fade"></div>
	</Fit>
</div>

<style>
	.card-visual {
		background:
			radial-gradient(ellipse 70% 60% at 50% 0%, rgba(110, 140, 255, 0.09), transparent),
			var(--paper);
	}

	.popover {
		position: absolute;
		left: 55px;
		top: 22px;
		width: 396px;
	}

	.fade {
		position: absolute;
		inset: auto 0 0;
		height: 60px;
		background: linear-gradient(transparent, var(--paper) 92%);
	}
</style>
