<!-- A reader page's <head>: title, description, canonical address, link previews and JSON-LD. -->
<script lang="ts">
	import { SITE_NAME, SITE_URL } from '#lib/config.js';
	import { jsonLd, type PageMeta } from './meta';

	interface Props extends PageMeta {
		/** The tab's title when it follows the page (the reader's chapter as it scrolls). */
		tabTitle?: string;
		type?: 'article' | 'website';
	}

	let { title, description, path, crumbs, tabTitle, type = 'article' }: Props = $props();

	const url = $derived(`${SITE_URL}${path}`);
	const ld = $derived(jsonLd({ title, description, path, crumbs }));
</script>

<svelte:head>
	<title>{tabTitle ?? title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={url} />
	<meta property="og:type" content={type} />
	<meta property="og:site_name" content={SITE_NAME} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={url} />
	<meta property="og:image" content="{SITE_URL}/og.jpg" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html ld}
</svelte:head>
