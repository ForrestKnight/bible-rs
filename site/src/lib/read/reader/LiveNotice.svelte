<!--
	Under a chapter of the NLT (../nlt.ts): Tyndale's credit line, which its terms ask for with the
	text, linked to the NLT's site; then the chapters either side, since a translation read live
	shows one chapter at a time rather than scrolling on into the next.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { CHAPTERS } from '../core';
	import { NLT_CREDIT, NLT_SITE } from '../nlt';
	import { follow } from '../ui/follow';

	let { index }: { index: number } = $props();

	const app = getApp();
	const turns = $derived(
		[
			{ to: index - 1, rel: 'prev' as const },
			{ to: index + 1, rel: 'next' as const }
		].filter((t) => t.to >= 0 && t.to < CHAPTERS)
	);
</script>

<footer class="live">
	<p class="credit">
		<a href={NLT_SITE} target="_blank" rel="noopener">{NLT_CREDIT}</a>
	</p>
	<nav class="turns" aria-label="Chapters">
		{#each turns as t (t.rel)}
			<a
				class="follow {t.rel}"
				href={app.chapterHref(t.to)}
				rel="{t.rel} nofollow"
				use:follow={() => app.dispatch({ type: 'jump', chapter: t.to })}
				>{t.rel === 'prev' ? '‹ ' : ''}{app.chapterName(t.to)}{t.rel === 'next' ? ' ›' : ''}</a
			>
		{/each}
	</nav>
</footer>

<style>
	.live {
		margin-top: calc(40 * var(--u));
		padding-top: calc(16 * var(--u));
		border-top: 1px solid var(--separator);
		font-family: var(--font-ui);
	}

	.credit {
		font-size: round(calc(11 * var(--u)), 1px);
		line-height: calc(16 * var(--u));
		color: var(--ink-3);
		text-align: center;
		text-wrap: pretty;
	}

	.credit a:hover {
		color: var(--ink-2);
		text-decoration: underline;
	}

	.turns {
		display: flex;
		justify-content: space-between;
		margin-top: calc(24 * var(--u));
	}

	.turns a {
		font-weight: 600;
		font-size: round(calc(11 * var(--u)), 1px);
		line-height: calc(16 * var(--u));
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--accent);
		padding: 8px 0;
	}

	.turns .next {
		margin-left: auto;
	}
</style>
