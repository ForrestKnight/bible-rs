<!-- Every topic's page (+page.ts), as a list of links with each one's number of verses. -->
<script lang="ts">
	import Frame from '#lib/read/seo/Frame.svelte';
	import Meta from '#lib/read/seo/Meta.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<Meta {...data.meta} type="website" />

<Frame crumbs={data.meta.crumbs} reader={{ href: '/read', label: 'Open the reader' }}>
	<header class="head">
		<h1>Bible Verses by Topic</h1>
		<p class="meta">{data.summary}</p>
	</header>

	<ul class="topics">
		{#each data.topics as t (t.href)}
			<li>
				<a href={t.href}><span class="name">{t.name}</span><span class="count">{t.count} verses</span></a>
			</li>
		{/each}
	</ul>
</Frame>

<style>
	.head {
		margin-bottom: 32px;
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

	.topics {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		column-gap: 24px;
	}

	.topics a {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
		padding: 12px 0;
		border-bottom: 1px solid var(--separator);
	}

	.name {
		color: var(--accent);
		font: var(--type-ui-strong);
	}

	.count {
		color: var(--ink-3);
		font: var(--type-small);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.topics a:hover .name {
		text-decoration: underline;
	}

	@media (max-width: 700px) {
		h1 {
			font-size: 36px;
			line-height: 44px;
		}
	}
</style>
