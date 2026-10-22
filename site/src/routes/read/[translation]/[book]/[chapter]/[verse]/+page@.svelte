<!--
	A verse's own page (+page.ts): the verse, the two either side, the topics that list it, the verse
	in every translation and the way on to its neighbours, its chapter, the Mac app or the clipboard.
	Its own layout (@): the reader's is the whole app.
-->
<script lang="ts">
	import Capsules from '#lib/read/seo/Capsules.svelte';
	import Frame from '#lib/read/seo/Frame.svelte';
	import Meta from '#lib/read/seo/Meta.svelte';
	import Runs from '#lib/read/seo/Runs.svelte';
	import { Icon } from '#lib/read/ui/index.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// The page has no app: Copy is this script alone.
	const copyScript = `<script>
document.addEventListener('click', function (e) {
	var b = e.target.closest && e.target.closest('[data-copy]');
	if (!b || !navigator.clipboard) return;
	navigator.clipboard.writeText(b.getAttribute('data-copy')).then(function () {
		var l = b.querySelector('.label');
		l.textContent = 'Copied';
		setTimeout(function () { l.textContent = 'Copy'; }, 1600);
	});
});
<\/script>`;
</script>

<Meta {...data.meta} />

<Frame crumbs={data.meta.crumbs} reader={{ href: data.chapter.href, label: `Read ${data.chapter.label}` }} notice={data.info.copyright}>
	<article>
		<header class="head">
			<h1>{data.label}</h1>
			<p class="version">{data.info.name} ({data.info.abbrev})</p>
		</header>
		<blockquote class="text">
			{#if data.verse.title}<p class="title">{data.verse.title}</p>{/if}
			<p><Runs runs={data.verse.runs} /></p>
		</blockquote>
		<div class="actions">
			<a class="button primary" href={data.app}>
				<Icon name="external" size={16} heavy />
				Open in Mac app
			</a>
			<button type="button" class="button" data-copy={data.copy}>
				<Icon name="copy" size={16} heavy />
				<span class="label">Copy</span>
			</button>
		</div>

		<section aria-labelledby="context">
			<h2 id="context">In context</h2>
			<div class="context">
				{#each data.context as v (v.number)}
					<p class:current={v.current} aria-current={v.current || undefined}>
						{#if v.current}<span class="vn">{v.number}</span>{:else}<a class="vn" href={v.href}>{v.number}</a>{/if}
						<Runs runs={v.runs} />
					</p>
				{/each}
			</div>
			<a class="more" href={data.chapter.href}>Read the whole of {data.chapter.label}</a>
		</section>

		{#if data.topics.length}
			<section class="topics" aria-labelledby="topics">
				<h2 id="topics">Topics</h2>
				<Capsules links={data.topics} />
			</section>
		{/if}

		<nav class="steps" aria-label="Verses">
			{#if data.prev}
				<a class="step" href={data.prev.href} rel="prev">
					<Icon name="chevron-left" size={16} heavy />
					{data.prev.label}
				</a>
			{/if}
			{#if data.next}
				<a class="step next" href={data.next.href} rel="next">
					{data.next.label}
					<Icon name="chevron-right" size={16} heavy />
				</a>
			{/if}
		</nav>

		<section aria-labelledby="translations">
			<h2 id="translations">{data.label} in every translation</h2>
			<dl class="others">
				{#each data.others as t (t.id)}
					<div class:current={t.id === data.info.id}>
						<dt>
							{#if t.id === data.info.id}<span class="abbrev">{t.abbrev}</span>{:else}<a class="abbrev" href={t.href}
									>{t.abbrev}</a
								>{/if}
							<span class="name">{t.name}</span>
						</dt>
						<dd>
							{#if t.runs}<Runs runs={t.runs} />{:else}<span class="missing">This translation leaves the verse out.</span>{/if}
						</dd>
					</div>
				{/each}
			</dl>
		</section>
	</article>
</Frame>

<!-- eslint-disable-next-line svelte/no-at-html-tags -->
{@html copyScript}

<style>
	.head {
		margin-bottom: 24px;
	}

	h1 {
		font: var(--type-display);
		letter-spacing: var(--type-display-tracking);
		font-variation-settings: 'opsz' 48;
	}

	.version {
		margin-top: 4px;
		color: var(--ink-2);
		font: var(--type-ui);
	}

	.text {
		margin: 0;
		font-family: var(--font-reading);
		font-size: 26px;
		line-height: 1.5;
		font-variation-settings: 'opsz' 24;
	}

	.text .title {
		margin-bottom: 8px;
		color: var(--ink-2);
		font-size: 17px;
		font-style: italic;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin: 28px 0 48px;
	}

	.button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 36px;
		padding: 0 16px;
		border-radius: 18px;
		background: var(--fill);
		color: var(--ink);
		font: 500 13px/20px var(--font-ui);
		cursor: pointer;
		transition: background-color var(--motion-fast);
	}

	.button:hover {
		background: var(--fill-strong);
	}

	.button.primary {
		background: var(--accent);
		color: var(--on-accent);
	}

	.button.primary:hover {
		background: linear-gradient(rgb(0 0 0 / 0.08), rgb(0 0 0 / 0.08)), var(--accent);
	}

	h2 {
		margin-bottom: 16px;
		color: var(--ink-3);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.context p {
		margin: 0 -12px;
		padding: 6px 12px;
		border-radius: var(--radius-sm);
		font: var(--type-snippet);
		font-size: 18px;
		line-height: 30px;
		color: var(--ink-2);
	}

	.context p.current {
		background: var(--selection);
		color: var(--ink);
	}

	.vn {
		margin-right: 4px;
		color: var(--verse);
		font: 600 11px/1 var(--font-ui);
		vertical-align: top;
		position: relative;
		top: 4px;
	}

	a.vn:hover {
		color: var(--accent);
	}

	.more {
		display: inline-block;
		margin-top: 12px;
		color: var(--accent);
		font: var(--type-ui-strong);
	}

	.topics {
		margin-top: 40px;
	}

	.steps {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		margin: 40px 0 48px;
		padding: 16px 0;
		border-block: 1px solid var(--separator);
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

	.others > div {
		padding: 14px 0;
		border-bottom: 1px solid var(--separator);
	}

	.others > div:last-child {
		border-bottom: 0;
	}

	dt {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 4px;
	}

	.abbrev {
		font: var(--type-ui-strong);
		color: var(--accent);
	}

	.current .abbrev {
		color: var(--ink);
	}

	.name {
		color: var(--ink-3);
		font: var(--type-small);
	}

	dd {
		margin: 0;
		font: var(--type-snippet);
		font-size: 17px;
		line-height: 28px;
	}

	.missing {
		color: var(--ink-3);
		font-style: italic;
	}

	@media (max-width: 700px) {
		h1 {
			font-size: 36px;
			line-height: 44px;
		}

		.text {
			font-size: 22px;
		}

		.actions {
			margin: 24px 0 40px;
		}
	}
</style>
