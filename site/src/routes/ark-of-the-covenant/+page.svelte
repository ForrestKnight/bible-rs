<!--
	The Ark of the Covenant (+page.server.ts): the answer, the 3D viewer, the dimensions in both
	cubits, every part with its KJV verses and whether the text gives its form, the questions, and
	links to every verse and chapter it cites. Prerendered; the viewer loads three.js after it paints.
-->
<script lang="ts">
	import { REPO_URL } from '#lib/config.js';
	import Viewer from '#lib/objects/Viewer.svelte';
	import { chapterLink, cubitsLabel, partRefs, refLabel, verseLink, type Dim } from '#lib/objects/spec.js';
	import Capsules from '#lib/read/seo/Capsules.svelte';
	import Frame from '#lib/read/seo/Frame.svelte';
	import Meta from '#lib/read/seo/Meta.svelte';
	import { Badge } from '#lib/read/ui/index.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const spec = $derived(data.spec);
	const part = (id: string) => spec.parts.find((p) => p.id === id)!;
	const chest = $derived(part('chest'));
	const seat = $derived(part('mercy-seat'));
	const rows = $derived([
		{ what: 'Length of the ark', d: chest.dims.length },
		{ what: 'Breadth of the ark', d: chest.dims.breadth },
		{ what: 'Height of the ark', d: chest.dims.height },
		{ what: 'Length of the mercy seat', d: seat.dims.length },
		{ what: 'Breadth of the mercy seat', d: seat.dims.breadth }
	]);
	const units = $derived(data.cubits.cubits);
	const both = (n: number) => units.map((c) => `${Number((n * c.inches).toFixed(1))} in · ${Math.round(n * c.cm)} cm`);

	/** "count" or cubits, as the reader would say it. */
	const value = (d: Dim) => (d.unit === 'count' ? String(d.value) : `${d.value} cubit${d.value === 1 ? '' : 's'}`);
	const dimName = (key: string) => key.replace(/^./, (c) => c.toUpperCase());

	/** objects/LICENSE: what to credit when reusing the spec or the model's images. */
	const ATTRIBUTION = `Ark of the Covenant specs and 3D models © 2026 Micky (github.com/michaelshimeles) / bible-rs contributors, licensed CC BY 4.0, ${REPO_URL}`;

	const passages = [
		{ name: 'Exodus 25', href: chapterLink('Exod.25.10', 'kjv') },
		{ name: 'Exodus 37', href: chapterLink('Exod.37.1', 'kjv') },
		{ name: 'Exodus 31:18', href: verseLink('Exod.31.18', 'kjv') },
		{ name: 'Hebrews 9:4', href: verseLink('Heb.9.4', 'kjv') },
		{ name: 'Ezekiel 40:5', href: verseLink('Ezek.40.5', 'kjv') }
	];
	const sourceVerses = $derived(
		spec.sources.flatMap((s) => {
			const [head, end] = s.split('-');
			const [code, chapter, first] = head.split('.');
			return Array.from({ length: Number(end ?? first) - Number(first) + 1 }, (_, i) => `${code}.${chapter}.${Number(first) + i}`);
		})
	);
</script>

<Meta {...data.meta} />

<Frame crumbs={data.meta.crumbs} reader={{ href: chapterLink('Exod.25.10', 'kjv'), label: 'Read Exodus 25' }} notice={data.notice} scripted>
	<header class="head">
		<h1>The Ark of the Covenant</h1>
		<p class="meta">Its dimensions, a 3D model built from Exodus 25 and 37, and every verse behind it</p>
	</header>
	<p class="answer">
		{data.answer} It was a chest of acacia wood overlaid with pure gold, with a lid of pure gold, the mercy seat, and two
		gold cherubim on the lid.
	</p>

	<div class="wide" id="model">
		<Viewer
			spec={data.spec}
			cubits={data.cubits}
			kjv={data.kjv}
			versesUrl="/{spec.slug}/verses.json"
			poster="/{spec.slug}/ark.jpg"
			imported={(id, refs) => import('#lib/objects/imported.js').then((m) => m.importedTexts(id, refs))}
		/>
	</div>

	<section aria-labelledby="dimensions">
		<h2 id="dimensions">Dimensions</h2>
		<div class="table">
			<table>
				<thead>
					<tr>
						<th scope="col">Measurement</th>
						<th scope="col">Cubits</th>
						{#each units as c (c.id)}<th scope="col">{c.label} ({c.inches} in)</th>{/each}
						<th scope="col">Verse</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as r (r.what)}
						<tr>
							<th scope="row">{r.what}</th>
							<td>{cubitsLabel(r.d.value)}</td>
							{#each both(r.d.value) as v, i (i)}<td>{v}</td>{/each}
							<td>{#if r.d.ref}<a href={verseLink(r.d.ref, 'kjv')}>{refLabel(r.d.ref)}</a>{/if}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="small">
			The text gives cubits only, so the inches and centimetres are estimates. A cubit is a forearm's length:
			{#each units as c, i (c.id)}{#if i}, or {/if}about {c.inches} inches for the {c.label.toLowerCase()}{#if c.ref}
					(<a href={verseLink(c.ref, 'kjv')}>{refLabel(c.ref)}</a>){/if}{/each}.
		</p>
	</section>

	<section aria-labelledby="parts">
		<h2 id="parts">Every part, from the text</h2>
		<p class="lead">
			Each part of the model comes from the verses below. Where the text gives a part's form, it is marked stated in the text;
			where the model has to choose, it is marked an interpretive choice, with what the text leaves open.
		</p>
		{#each spec.parts as p, i (p.id)}
			<article class="part" id={p.id}>
				<header>
					<h3><span class="n" aria-hidden="true">{i + 1}</span>{p.label}</h3>
					<Badge kind={p.interpretive ? 'outline' : 'success'} label={p.interpretive ? 'Interpretive choice' : 'Stated in the text'} />
				</header>
				{#each partRefs(p) as ref (ref)}
					<blockquote>
						<p>{data.kjv.verses[ref]}</p>
						<cite><a href={verseLink(ref, 'kjv')}>{refLabel(ref)} (KJV)</a></cite>
					</blockquote>
				{/each}
				<p class="note">{p.note}</p>
				{#if Object.keys(p.dims).length}
					<dl class="dims">
						{#each Object.entries(p.dims) as [key, d] (key)}
							<div>
								<dt>{dimName(key)}</dt>
								<dd>
									{value(d)}
									{#if d.interpretive}<span class="interp">interpretive: {d.note}</span>{:else if d.ref}<a href={verseLink(d.ref, 'kjv')}>{refLabel(d.ref)}</a>{/if}
								</dd>
							</div>
						{/each}
					</dl>
				{/if}
				{#each p.options ?? [] as o (o.id)}
					<p class="note">
						{o.label}, debated: {o.choices.map((c) => `${c.label.toLowerCase()} (${c.note.replace(/\.$/, '')})`).join('; or ')}. The model
						shows {o.choices.find((c) => c.id === o.default)?.label.toLowerCase()} and can switch.
					</p>
				{/each}
			</article>
		{/each}
	</section>

	<section aria-labelledby="faq">
		<h2 id="faq">Questions</h2>
		{#each data.faq as f (f.question)}
			<div class="qa">
				<h3>{f.question}</h3>
				<p>{f.answer}</p>
				<p class="refs">
					{#each f.refs as ref, i (ref)}{#if i}, {/if}<a href={verseLink(ref, 'kjv')}>{refLabel(ref)}</a>{/each}
				</p>
			</div>
		{/each}
	</section>

	<section aria-labelledby="read">
		<h2 id="read">Read the passages</h2>
		<Capsules links={passages} />
		<p class="verses">
			{#each sourceVerses as ref, i (ref)}{#if i}<span aria-hidden="true"> · </span>{/if}<a href={verseLink(ref, 'kjv')}
					>{refLabel(ref)}</a
				>{/each}
		</p>
	</section>

	<section aria-labelledby="spec" class="spec">
		<h2 id="spec">Built from an open spec</h2>
		<p>
			Every number in this model lives in <a href="{REPO_URL}/blob/main/objects/ark.json">objects/ark.json</a>, with the verse it
			comes from and the KJV and BSB words that give it, or a note saying why it's an interpretive choice. A check runs on every
			change and finds each quoted phrase in its verse. Think something is wrong?
			<a href="{REPO_URL}/blob/main/objects/CONTRIBUTING.md">Propose a correction</a>.
		</p>
		<p class="license">
			The spec and the model's images are licensed
			<a href="https://creativecommons.org/licenses/by/4.0/" rel="license">CC BY 4.0</a>; the code is MIT. Credit them as:
			{ATTRIBUTION}
		</p>
	</section>
</Frame>

<style>
	.head {
		margin-bottom: 20px;
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

	.answer {
		margin-bottom: 28px;
		font-family: var(--font-reading);
		font-size: 19px;
		line-height: 1.6;
		font-variation-settings: 'opsz' 18;
	}

	/* The viewer is wider than the text column, up to 1120 px, centred on it. */
	.wide {
		width: min(1120px, calc(100vw - 48px));
		margin: 0 0 48px calc(50% - min(560px, 50vw - 24px));
	}

	section {
		margin-top: 40px;
	}

	/* Over the reader's reset (read.css), which is as specific as a bare element here. */
	section h2 {
		margin-bottom: 16px;
		font: var(--type-ui-title);
		font-size: 22px;
		line-height: 28px;
		letter-spacing: var(--type-ui-title-tracking);
	}

	.lead,
	.small {
		color: var(--ink-2);
		font: var(--type-ui);
	}

	.small {
		margin-top: 10px;
		font: var(--type-small);
	}

	.lead {
		margin-bottom: 8px;
	}

	.table {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font: var(--type-ui);
		font-variant-numeric: tabular-nums;
	}

	th,
	td {
		padding: 8px 10px 8px 0;
		border-bottom: 1px solid var(--separator);
		text-align: left;
		white-space: nowrap;
	}

	thead th {
		color: var(--ink-2);
		font: var(--type-small);
		font-weight: 600;
	}

	tbody th {
		font-weight: 500;
		white-space: normal;
	}

	a {
		color: var(--accent);
	}

	/* Links inside running text are underlined, not only coloured. */
	p a {
		text-decoration: underline;
		text-decoration-thickness: 1px;
		text-underline-offset: 2px;
	}

	.part {
		padding: 20px 0;
		border-top: 1px solid var(--separator);
	}

	.part header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 8px;
	}

	section h3 {
		display: flex;
		align-items: center;
		gap: 8px;
		font: var(--type-ui-strong);
		font-size: 17px;
		line-height: 24px;
	}

	.n {
		display: inline-grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 1.5px solid var(--accent);
		color: var(--accent);
		font: 600 11px/1 var(--font-ui);
	}

	.part blockquote {
		margin: 10px 0;
		padding: 2px 0 2px 14px;
		border-left: 2px solid var(--accent);
		font: var(--type-snippet);
		font-size: 18px;
		line-height: 30px;
	}

	cite {
		display: block;
		margin-top: 2px;
		font: var(--type-small);
		font-style: normal;
	}

	.note {
		margin-top: 8px;
		color: var(--ink-2);
		font: var(--type-ui);
	}

	.dims {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 6px 16px;
		margin-top: 12px;
		font: var(--type-small);
	}

	.dims dt {
		color: var(--ink-2);
		font-weight: 600;
	}

	.dims dd {
		margin: 0;
	}

	.interp {
		display: block;
		color: var(--ink-2);
	}

	.qa {
		padding: 16px 0;
		border-top: 1px solid var(--separator);
	}

	.qa p {
		margin-top: 6px;
		font: var(--type-ui);
		font-size: 16px;
		line-height: 25px;
	}

	.qa .refs {
		font: var(--type-small);
	}

	.verses {
		margin-top: 16px;
		color: var(--ink-2);
		font: var(--type-small);
		line-height: 22px;
	}

	.spec p {
		font: var(--type-ui);
		font-size: 16px;
		line-height: 25px;
	}

	.spec .license {
		margin-top: 12px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	@media (max-width: 860px) {
		/* Edge to edge on a phone; the page's side padding is 20 px there. */
		.wide {
			width: auto;
			margin: 0 -20px 40px;
		}

		.wide :global(.controls),
		.wide :global(.panel) {
			padding: 0 20px;
		}
	}

	@media (max-width: 700px) {
		h1 {
			font-size: 36px;
			line-height: 44px;
		}

		.answer {
			font-size: 17px;
		}

		.part blockquote {
			font-size: 17px;
			line-height: 28px;
		}
	}
</style>
