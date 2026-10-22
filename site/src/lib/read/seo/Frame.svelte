<!--
	The frame of the reader's plain pages (a verse, a book, a translation, a topic, an object): no
	app, only text and links in the reader's type and colours, in the theme the reader was last set
	to (app.html applies it). They're prerendered with no JavaScript of their own (csr = false), but
	for an object's page, whose 3D viewer needs it (`scripted`).
-->
<script lang="ts">
	import '../read.css';
	import type { Snippet } from 'svelte';
	import { analyticsTag } from '#lib/analytics.js';
	import { NAME } from '#lib/config.js';
	import { TOPICS_PATH } from '#lib/topics/index.js';
	import { AppIcon, Icon } from '../ui';
	import type { Crumb } from './meta';

	interface Props {
		crumbs: Crumb[];
		/** Where the button at the top opens the reader, and what it says ("Read John 3"). */
		reader: { href: string; label: string };
		/** The small print at the foot: the translation's copyright, or each translation's. */
		notice?: string | string[];
		/** A page with SvelteKit's client (csr), whose +layout.ts already loads the analytics. */
		scripted?: boolean;
		children: Snippet;
	}

	let { crumbs, reader, notice, scripted = false, children }: Props = $props();
</script>

<svelte:head>
	<link rel="preload" href="/fonts/read/literata.woff2" as="font" type="font/woff2" crossorigin="anonymous" />
	<link rel="preload" href="/fonts/read/inter.woff2" as="font" type="font/woff2" crossorigin="anonymous" />
	{#if !scripted}
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html analyticsTag()}
	{/if}
</svelte:head>

<div class="frame">
	<header class="bar">
		<a class="brand" href="/">
			<AppIcon size={24} />
			<span>{NAME}</span>
		</a>
		<a class="open" href={reader.href}>
			<Icon name="book" size={16} />
			{reader.label}
		</a>
	</header>
	<main class="page">
		<nav aria-label="Breadcrumb">
			<ol class="crumbs">
				{#each crumbs as c, i (c.path)}
					<li>
						{#if i < crumbs.length - 1}<a href={c.path}>{c.name}</a>{:else}<span aria-current="page">{c.name}</span>{/if}
					</li>
				{/each}
			</ol>
		</nav>
		{@render children()}
	</main>
	<footer class="foot">
		{#each [notice ?? []].flat() as n, i (i)}<p>{n}</p>{/each}
		<p class="links">
			<a href="/">{NAME} for Mac</a>
			<a href="/read">Web reader</a>
			<a href={TOPICS_PATH}>Verses by topic</a>
			<a href="/privacy">Privacy</a>
		</p>
	</footer>
</div>

<style>
	/* The reader's window doesn't scroll; these pages do. */
	:global(html[data-read]),
	:global(html[data-read] body) {
		overflow: auto;
		overscroll-behavior: auto;
	}

	.frame {
		min-height: 100vh;
		background: var(--paper);
		color: var(--ink);
	}

	.bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		height: var(--titlebar-height);
		padding: 0 20px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 8px;
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
	}

	/* The icon's drop is drawn for 64 pt; at 24 it would smudge the bar. */
	.bar .brand :global(.icon) {
		box-shadow: inset 0 0 0 0.5px rgb(255 255 255 / 0.25);
	}

	.open {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: var(--titlebar-capsule-height);
		padding: 0 14px;
		border-radius: var(--titlebar-capsule-radius);
		background: var(--fill);
		font: var(--type-ui-strong);
		white-space: nowrap;
		transition: background-color var(--motion-fast);
	}

	.open:hover {
		background: var(--fill-strong);
	}

	.open :global(svg) {
		color: var(--accent);
	}

	.page {
		max-width: 720px;
		margin: 0 auto;
		padding: 16px 24px 64px;
	}

	.crumbs {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 8px;
		margin-bottom: 32px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	.crumbs li + li::before {
		content: '›';
		margin-right: 8px;
	}

	.crumbs a {
		color: var(--ink-2);
	}

	.crumbs a:hover {
		color: var(--accent);
	}

	.foot {
		max-width: 720px;
		margin: 0 auto;
		padding: 24px 24px 48px;
		border-top: 1px solid var(--separator);
		color: var(--ink-3);
		font: var(--type-small);
	}

	.foot p + p {
		margin-top: 8px;
	}

	.links {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 16px;
	}

	.links a {
		color: var(--ink-2);
	}

	.links a:hover {
		color: var(--accent);
	}

	@media (max-width: 700px) {
		.bar {
			padding: 0 16px;
		}

		.page {
			padding: 8px 20px 48px;
		}

		.foot {
			padding: 24px 20px 40px;
		}
	}
</style>
