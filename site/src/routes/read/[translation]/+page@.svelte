<!--
	A translation's page (+page.server.ts): what it is, where to start, its books by testament and
	the other translations. Its own layout (@): the reader's is the whole app.
-->
<script lang="ts">
	import Frame from '#lib/read/seo/Frame.svelte';
	import Meta from '#lib/read/seo/Meta.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<Meta {...data.meta} type="website" />

<Frame crumbs={data.meta.crumbs} reader={{ href: data.starts[0].href, label: 'Start reading' }} notice={data.info.copyright}>
	<header class="head">
		<h1>{data.info.name}</h1>
		<p class="meta">
			{data.info.abbrev}{#if data.info.year}&nbsp;· {data.info.year}{/if}{#if data.info.description}&nbsp;· {data.info
					.description}{/if}
		</p>
		<p class="license">{data.info.license}</p>
	</header>

	<nav aria-labelledby="start">
		<h2 id="start">Start reading</h2>
		<ul class="starts">
			{#each data.starts as s (s.href)}
				<li><a href={s.href}>{s.label}</a></li>
			{/each}
		</ul>
	</nav>

	{#each data.testaments as t (t.name)}
		<nav aria-label={t.name}>
			<h2>{t.name}</h2>
			<ul class="books">
				{#each t.books as b (b.href)}
					<li>
						<a href={b.href}>{b.name}</a>
						<span class="count">{b.chapters}</span>
					</li>
				{/each}
			</ul>
		</nav>
	{/each}

	<nav aria-labelledby="translations">
		<h2 id="translations">Other translations</h2>
		<ul class="others">
			{#each data.others as t (t.href)}
				<li><a href={t.href}><span class="abbrev">{t.abbrev}</span> {t.name}</a></li>
			{/each}
		</ul>
	</nav>
</Frame>

<style>
	.head {
		margin-bottom: 40px;
	}

	h1 {
		font: var(--type-display);
		letter-spacing: var(--type-display-tracking);
		font-variation-settings: 'opsz' 48;
	}

	.meta {
		margin-top: 12px;
		color: var(--ink-2);
		font: var(--type-ui);
	}

	.license {
		margin-top: 4px;
		color: var(--ink-3);
		font: var(--type-small);
	}

	h2 {
		margin-bottom: 16px;
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	nav + nav {
		margin-top: 40px;
	}

	.starts {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.starts a {
		display: inline-flex;
		align-items: center;
		height: 36px;
		padding: 0 16px;
		border-radius: 18px;
		background: var(--fill);
		font: 500 13px/20px var(--font-ui);
		transition: background-color var(--motion-fast);
	}

	.starts a:hover {
		background: var(--fill-strong);
	}

	.books {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 0 24px;
	}

	.books li {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		padding: 6px 0;
		border-bottom: 1px solid var(--separator);
		font: var(--type-ui);
	}

	.books a:hover,
	.others a:hover {
		color: var(--accent);
	}

	.count {
		color: var(--ink-3);
		font: var(--type-small);
		font-variant-numeric: tabular-nums;
	}

	.others {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 8px 24px;
		font: var(--type-ui);
	}

	.abbrev {
		display: inline-block;
		min-width: 3em;
		color: var(--accent);
		font-weight: 600;
	}

	@media (max-width: 700px) {
		h1 {
			font-size: 36px;
			line-height: 44px;
		}
	}
</style>
