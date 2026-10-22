<!--
	The Ark's viewer on its own, for the Mac app's webview (crates/app), which loads it from the app
	bundle with ?translation=<id>&theme=<theme>. The head applies the theme before the first paint.
-->
<script lang="ts">
	import '#lib/read/read.css';
	import { SITE_URL } from '#lib/config.js';
	import Viewer from '#lib/objects/Viewer.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const theme = `<script>try{var t=new URLSearchParams(location.search).get('theme');if(t==='light'||t==='sepia'||t==='dark')document.documentElement.dataset.theme=t}catch{}<\/script>`;
</script>

<svelte:head>
	<title>{data.spec.title}</title>
	<meta name="robots" content="noindex" />
	<link rel="canonical" href="{SITE_URL}/{data.spec.slug}" />
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- a constant script -->
	{@html theme}
</svelte:head>

<main class="embed">
	<Viewer
		spec={data.spec}
		cubits={data.cubits}
		kjv={data.kjv}
		versesUrl="/{data.spec.slug}/verses.json"
		poster="/{data.spec.slug}/ark.jpg"
		embed
	/>
</main>

<style>
	/* Wide, the viewer fills the window and its panel scrolls; narrow, the page scrolls (Viewer.svelte). */
	:global(html[data-read]),
	:global(html[data-read] body) {
		overflow: hidden;
	}

	@media (max-width: 860px) {
		:global(html[data-read]),
		:global(html[data-read] body) {
			overflow: auto;
		}
	}

	.embed {
		background: var(--paper);
		color: var(--ink);
		min-height: 100vh;
	}
</style>
