<script lang="ts">
	import type { Snippet } from 'svelte';
	import Footer from './Footer.svelte';
	import Nav from './Nav.svelte';

	let { heading, meta, children }: { heading: string; meta: string; children: Snippet } = $props();
</script>

<Nav />

<main class="doc">
	<header>
		<h1>{heading}</h1>
		<p class="meta">{meta}</p>
	</header>
	{@render children()}
</main>

<Footer />

<style>
	.doc {
		max-width: 680px;
		margin: 0 auto;
		padding: 48px 24px 72px;
		color: var(--muted);
		font-size: 16px;
		line-height: 26px;
	}

	header {
		padding-bottom: 12px;
	}

	h1 {
		margin: 0;
		color: var(--fg);
		font: 700 44px/1.05 var(--display);
		letter-spacing: -0.035em;
	}

	.meta {
		margin: 12px 0 0;
		font-size: 14px;
	}

	.doc :global(.lede) {
		margin: 24px 0 8px;
		color: var(--fg);
		font-size: 18px;
		line-height: 29px;
	}

	.doc :global(h2) {
		margin: 40px 0 8px;
		color: var(--fg);
		font: 600 20px/26px var(--display);
		letter-spacing: -0.02em;
	}

	.doc :global(p) {
		margin: 0 0 12px;
	}

	.doc :global(ul) {
		margin: 0 0 12px;
		padding-left: 20px;
	}

	.doc :global(li) {
		margin: 8px 0;
	}

	.doc :global(a) {
		color: var(--link);
	}

	.doc :global(strong) {
		color: var(--fg);
		font-weight: 600;
	}

	@media (max-width: 640px) {
		.doc {
			padding: 32px 16px 56px;
		}

		h1 {
			font-size: 36px;
		}
	}
</style>
