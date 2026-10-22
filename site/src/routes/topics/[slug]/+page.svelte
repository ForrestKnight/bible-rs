<!--
	A topic's page (+page.ts): Marketing's introduction, then each verse in the KJV and the BSB,
	each linked to its own page, and the related topics.
-->
<script lang="ts">
	import Capsules from '#lib/read/seo/Capsules.svelte';
	import Frame from '#lib/read/seo/Frame.svelte';
	import Meta from '#lib/read/seo/Meta.svelte';
	import Runs from '#lib/read/seo/Runs.svelte';
	import { TOPICS_PATH } from '#lib/topics/index.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<Meta {...data.meta} />

<Frame
	crumbs={data.meta.crumbs}
	reader={{ href: '/read', label: 'Open the reader' }}
	notice={data.translations.map((t) => t.copyright)}
>
	<header class="head">
		<h1>{data.heading}</h1>
		<p class="meta">
			{data.verses.length} verses in the {data.translations.map((t) => `${t.name} (${t.abbrev})`).join(' and the ')}
		</p>
	</header>
	<p class="intro">{data.intro}</p>

	<ol class="verses">
		{#each data.verses as v (v.ref)}
			<li>
				<h2><a href={v.texts[0].href}>{v.ref}</a></h2>
				<dl>
					{#each v.texts as t (t.id)}
						<div>
							<dt><a href={t.href} aria-label="{v.ref} in the {t.abbrev}">{t.abbrev}</a></dt>
							<dd><Runs runs={t.runs} /></dd>
						</div>
					{/each}
				</dl>
			</li>
		{/each}
	</ol>

	<section class="related" aria-labelledby="related">
		<h2 id="related">Related topics</h2>
		<Capsules links={data.related} />
		<a class="all" href={TOPICS_PATH}>All topics</a>
	</section>
</Frame>

<style>
	.head {
		margin-bottom: 24px;
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

	.intro {
		margin-bottom: 32px;
		font-family: var(--font-reading);
		font-size: 19px;
		line-height: 1.6;
		font-variation-settings: 'opsz' 18;
	}

	.verses > li {
		padding: 20px 0;
		border-top: 1px solid var(--separator);
	}

	.verses h2 {
		margin-bottom: 8px;
		font: var(--type-ui-strong);
	}

	.verses h2 a {
		color: var(--accent);
	}

	dl > div {
		display: grid;
		grid-template-columns: 3.5em 1fr;
		gap: 12px;
		align-items: baseline;
	}

	dl > div + div {
		margin-top: 8px;
	}

	dt a {
		color: var(--ink-3);
		font: var(--type-small);
		font-weight: 600;
	}

	dt a:hover {
		color: var(--accent);
	}

	dd {
		margin: 0;
		font: var(--type-snippet);
		font-size: 18px;
		line-height: 30px;
	}

	.related {
		margin-top: 16px;
		padding-top: 32px;
		border-top: 1px solid var(--separator);
	}

	.related h2 {
		margin-bottom: 16px;
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.all {
		display: inline-block;
		margin-top: 20px;
		color: var(--accent);
		font: var(--type-ui-strong);
	}

	@media (max-width: 700px) {
		h1 {
			font-size: 36px;
			line-height: 44px;
		}

		.intro {
			font-size: 17px;
		}

		dd {
			font-size: 17px;
			line-height: 28px;
		}
	}
</style>
