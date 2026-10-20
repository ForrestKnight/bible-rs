<script lang="ts">
	import Icon from '#lib/app/Icon.svelte';
	import { easeOutBack, envelope, span } from '#lib/ease.js';
	import { loop } from '#lib/motion.svelte.js';
	import { PSALM_23, PSALM_23_HEADING, type Run } from '#lib/scripture.js';
	import Fit from './Fit.svelte';

	let { paused = false }: { paused?: boolean } = $props();

	const clock = loop(6400, 3000, () => paused);
	let t = $derived(clock.t);

	let highlight = $derived(envelope(t, 700, 1100, 5600, 6100));
	let bookmark = $derived(Math.min(easeOutBack(span(t, 1400, 1800)), 1 - span(t, 5600, 6000)));
</script>

{#snippet runs(list: Run[])}
	{#each list as run, i (i)}<span class={{ sc: run.smallCaps }}>{run.text}</span>{/each}
{/snippet}

<div class="card-visual app" {@attach clock.observe}>
	<Fit width={506} height={304}>
		<div class="page">
			<div class="label">PSALM 23</div>
			<h4>{@render runs(PSALM_23_HEADING)}</h4>
			<div class="title">A Psalm of David.</div>
			{#each PSALM_23 as verse (verse.n)}
				{#each verse.lines as line, i (i)}
					<div class={['line', { indent: line.indent > 0 }]}>
						{#if i === 0}<span class="n">{verse.n}</span>{/if}
						<span
							class="text"
							style:background-color={verse.n === 1
								? `color-mix(in srgb, var(--hl-green) ${highlight * 100}%, transparent)`
								: undefined}>{@render runs(line.text)}</span
						>
					</div>
				{/each}
			{/each}
		</div>
		<div class="bookmark" style:opacity={Math.max(0, bookmark)} style:scale={Math.max(0, bookmark)}>
			<Icon name="bookmark" size={13} />
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
		top: 28px;
		color: var(--ink);
		font: 14px/24px var(--serif);
	}

	.label {
		padding-left: 32px;
		color: var(--accent);
		font: 600 9px/13px var(--sans);
		letter-spacing: 0.12em;
	}

	h4 {
		margin: 2px 0 0;
		padding-left: 32px;
		font: 400 22px/30px var(--serif);
	}

	.title {
		padding: 0 0 8px 32px;
		color: var(--ink-2);
		font: italic 12px/18px var(--serif);
	}

	.line {
		position: relative;
		padding-left: 32px;
	}

	.line.indent {
		padding-left: 52px;
	}

	.n {
		position: absolute;
		left: 0;
		width: 22px;
		color: var(--verse);
		font: 600 9.5px/24px var(--sans);
		text-align: right;
	}

	.text {
		border-radius: 3px;
	}

	.sc {
		font-variant-caps: small-caps;
	}

	.bookmark {
		position: absolute;
		left: 452px;
		top: 116px;
		color: var(--accent);
	}

	.fade {
		position: absolute;
		inset: auto 0 0;
		height: 72px;
		background: linear-gradient(transparent, var(--paper));
	}
</style>
