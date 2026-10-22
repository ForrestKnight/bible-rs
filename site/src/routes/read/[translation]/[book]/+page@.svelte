<!--
	A book's page (+page.server.ts): its chapters as a grid and as a list with each one's opening
	words, the book in the other translations, and the books either side. Its own layout (@): the
	reader's is the whole app.
-->
<script lang="ts">
	import Frame from '#lib/read/seo/Frame.svelte';
	import Meta from '#lib/read/seo/Meta.svelte';
	import { Icon } from '#lib/read/ui/index.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<Meta {...data.meta} />

<Frame crumbs={data.meta.crumbs} reader={{ href: data.chapters[0].href, label: `Read ${data.chapters[0].label}` }} notice={data.info.copyright}>
	<header class="head">
		<h1>{data.book.name}</h1>
		{#if data.book.title && data.book.title !== data.book.name}<p class="subtitle">{data.book.title}</p>{/if}
		<p class="meta">
			{data.info.name} ({data.info.abbrev}) · {data.book.chapters === 1 ? '1 chapter' : `${data.book.chapters} chapters`}
		</p>
	</header>

	<nav aria-labelledby="chapters">
		<h2 id="chapters">Chapters</h2>
		<ol class="grid">
			{#each data.chapters as ch (ch.n)}
				<li><a href={ch.href} aria-label={ch.label}>{ch.n}</a></li>
			{/each}
		</ol>
	</nav>

	<section aria-labelledby="openings">
		<h2 id="openings">How each chapter begins</h2>
		<ol class="openings">
			{#each data.chapters as ch (ch.n)}
				<li>
					<a href={ch.href}>{ch.label}</a>
					<p>{ch.opening}</p>
				</li>
			{/each}
		</ol>
	</section>

	<nav class="steps" aria-label="Books">
		{#if data.prev}
			<a class="step" href={data.prev.href} rel="prev">
				<Icon name="chevron-left" size={16} heavy />
				{data.prev.name}
			</a>
		{/if}
		{#if data.next}
			<a class="step next" href={data.next.href} rel="next">
				{data.next.name}
				<Icon name="chevron-right" size={16} heavy />
			</a>
		{/if}
	</nav>

	<section aria-labelledby="translations">
		<h2 id="translations">{data.book.name} in other translations</h2>
		<ul class="others">
			{#each data.others as t (t.href)}
				<li><a href={t.href}><span class="abbrev">{t.abbrev}</span> {t.name}</a></li>
			{/each}
		</ul>
	</section>
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

	.subtitle {
		margin-top: 4px;
		color: var(--ink-2);
		font: var(--type-subtitle);
		font-style: italic;
	}

	.meta {
		margin-top: 12px;
		color: var(--ink-2);
		font: var(--type-ui);
	}

	h2 {
		margin-bottom: 16px;
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(40px, 1fr));
		gap: 8px;
		margin-bottom: 48px;
	}

	.grid a {
		display: grid;
		place-items: center;
		height: 40px;
		border-radius: var(--radius-sm);
		background: var(--fill);
		font: var(--type-ui-strong);
		font-variant-numeric: tabular-nums;
		transition: background-color var(--motion-fast);
	}

	.grid a:hover {
		background: var(--accent);
		color: var(--on-accent);
	}

	.openings li {
		padding: 12px 0;
		border-bottom: 1px solid var(--separator);
	}

	.openings a {
		color: var(--accent);
		font: var(--type-ui-strong);
	}

	.openings p {
		margin-top: 2px;
		font: var(--type-snippet);
		color: var(--ink-2);
	}

	.steps {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		margin: 32px 0 48px;
	}

	.step {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--accent);
		font: var(--type-ui-strong);
	}

	.step.next {
		margin-left: auto;
	}

	.others {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 8px 24px;
		font: var(--type-ui);
	}

	.others a:hover {
		color: var(--accent);
	}

	.abbrev {
		display: inline-block;
		min-width: 3em;
		color: var(--accent);
		font-weight: 600;
	}

	@media (max-width: 700px) {
		h1 {
			font-size: 40px;
			line-height: 48px;
		}
	}
</style>
