<script lang="ts">
	import Palette from '#lib/app/Palette.svelte';
	import type { Place } from '#lib/demo/script.js';
	import { envelope } from '#lib/ease.js';
	import { loop } from '#lib/motion.svelte.js';
	import Fit from './Fit.svelte';

	const QUERY = 'revalation 21';
	const KEYS = [...QUERY].map((_, i) => 500 + i * 110);
	const DONE = KEYS[KEYS.length - 1] + 120;
	const ENTER = 3600;
	const PLACE: Place = {
		ref: [
			['Rev', true],
			['elation ', false],
			['21', true]
		],
		detail: 'Chapter · 27 verses',
		kind: 'chapter'
	};

	let { paused = false }: { paused?: boolean } = $props();

	const clock = loop(7000, 3200, () => paused);
	let t = $derived(clock.t);

	let shown = $derived(envelope(t, 0, 250, 5800, 6300));
	let query = $derived(QUERY.slice(0, KEYS.filter((k) => t >= k).length));
	let typing = $derived(KEYS.some((k) => t >= k && t < k + 450));
	let commands = $derived(
		query
			? [
					{ icon: 'search' as const, label: `Search for “${query}”`, keys: ['⌘', 'F'] },
					...(t >= DONE ? [{ icon: 'parallel' as const, label: 'Open Revelation 21 in parallel', keys: ['⇧', '⌘', 'P'] }] : [])
				]
			: []
	);
</script>

<div class="card-visual app" {@attach clock.observe}>
	<Fit width={506} height={304}>
		<div class="palette" style:opacity={shown} style:transform="translateY({(1 - shown) * 8}px)">
			<Palette
				{query}
				caret={typing || Math.floor(t / 530) % 2 === 0}
				places={t >= DONE ? [PLACE] : []}
				{commands}
				pressed={t >= ENTER && t < ENTER + 160}
			/>
		</div>
	</Fit>
</div>

<style>
	.card-visual {
		background:
			radial-gradient(ellipse 70% 60% at 50% 0%, rgba(110, 140, 255, 0.09), transparent),
			var(--paper);
	}

	.palette {
		position: absolute;
		left: 43px;
		top: 30px;
		width: 420px;
	}
</style>
