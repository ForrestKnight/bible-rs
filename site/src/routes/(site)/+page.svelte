<script lang="ts">
	import { resolve } from '$app/paths';
	import DownloadCount from '#lib/DownloadCount.svelte';
	import Footer from '#lib/Footer.svelte';
	import Nav from '#lib/Nav.svelte';
	import CompareCard from '#lib/cards/CompareCard.svelte';
	import JumpCard from '#lib/cards/JumpCard.svelte';
	import SearchCard from '#lib/cards/SearchCard.svelte';
	import StudyCard from '#lib/cards/StudyCard.svelte';
	import ThemesCard from '#lib/cards/ThemesCard.svelte';
	import TypesetCard from '#lib/cards/TypesetCard.svelte';
	import { ARCH, MIN_MACOS, NAME, REPO_URL, SITE_NAME, SITE_URL, VERSION } from '#lib/config.js';
	import Showcase from '#lib/demo/Showcase.svelte';
	import { Player } from '#lib/demo/player.svelte.js';
	import { play } from '#lib/demo/sound.js';
	import { reducedMotion, reveal } from '#lib/motion.svelte.js';

	const title = `${NAME}: a fast, native reader for the Mac | ${SITE_NAME}`;
	const description =
		'A very fast, native Bible reader for the Mac. Beautifully typeset, searchable as you type, with highlights, notes and bookmarks kept on your computer.';
	const url = `${SITE_URL}/`;
	const image = `${SITE_URL}/og.jpg`;
	const imageAlt = `The ${NAME} app icon and name beside the reader open at John 1`;

	const app = JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'SoftwareApplication',
		name: NAME,
		description,
		url,
		image,
		operatingSystem: 'macOS',
		applicationCategory: 'ReferenceApplication',
		offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
		downloadUrl: `${SITE_URL}/download`,
		softwareVersion: VERSION
	}).replaceAll('<', '\\u003c');
	// A literal closing tag here would end this <script> block.
	const jsonLd = `<script type="application/ld+json">${app}<\/script>`;

	const player = new Player();

	let paused: Record<string, boolean> = $state({});

	const reading = [
		{
			title: 'Typeset for reading',
			body: 'Literata at a comfortable measure, with superscript verse numbers, poetry that keeps its lines, LORD in small capitals and the words of Christ in red. Paragraphs or a verse per line.',
			Visual: TypesetCard
		},
		{
			title: 'Light, sepia and dark',
			body: 'Three themes made for long reading, from white paper to warm sepia to a soft dark. ⇧⌘L switches between them, and the size, spacing and measure are yours to set.',
			Visual: ThemesCard
		}
	];

	const study = [
		{
			title: 'Jump anywhere',
			body: 'Press ⌘K and type a reference the way you’d write it: jn 3:16, Rom 8, 1co13, even revalation 21. Recent places, translations and commands come up too.',
			Visual: JumpCard
		},
		{
			title: 'Search as you type',
			body: 'Every verse is searched as you type, in under 2 ms. Quote a phrase, put a minus before a word to leave it out, and search the whole Bible, one testament or one book.',
			Visual: SearchCard
		},
		{
			title: 'Highlights and notes',
			body: 'Highlight in six colours, write notes in the margin, bookmark verses and follow a reading plan. It all stays in a file on your Mac, and exports to Markdown or JSON.',
			Visual: StudyCard
		},
		{
			title: 'Translations side by side',
			body: 'Seven public-domain translations are built in. Read them in parallel columns, or compare one verse across all of them. Add the NLT and NET free, the ESV with a free key, or import a file you own.',
			Visual: CompareCard
		}
	];

	const shortcuts = [
		{ keys: ['⌘', 'K'], label: 'Jump to a passage', primary: true },
		{ keys: ['⌘', 'F'], label: 'Search' },
		{ keys: ['←', '→'], label: 'Previous or next chapter' },
		{ keys: ['⌘[', '⌘]'], label: 'Back and forward' },
		{ keys: ['J', 'K'], label: 'Move between verses' },
		{ keys: ['1–6'], label: 'Highlight in a colour' },
		{ keys: ['N'], label: 'Write a note' },
		{ keys: ['C'], label: 'Compare translations' },
		{ keys: ['⇧', '⌘', 'P'], label: 'Read in parallel' },
		{ keys: ['⇧', '⌘', 'L'], label: 'Change theme' }
	];
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={url} />
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content={SITE_NAME} />
	<meta property="og:locale" content="en_US" />
	<meta property="og:url" content={url} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:image" content={image} />
	<meta property="og:image:type" content="image/jpeg" />
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />
	<meta property="og:image:alt" content={imageAlt} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={image} />
	<meta name="twitter:image:alt" content={imageAlt} />
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html jsonLd}
</svelte:head>

{#snippet apple()}
	<svg viewBox="0 0 17 21" aria-hidden="true">
		<path
			d="M11.2 4.3c.6-.8 1-1.8.9-2.8-.9 0-2 .6-2.6 1.4-.6.7-1.1 1.7-.9 2.7 1 .1 2-.5 2.6-1.3zM12.1 5.8c-1.4-.1-2.7.8-3.3.8-.7 0-1.7-.8-2.9-.8C4.4 5.9 3 6.8 2.2 8.1c-1.6 2.7-.4 6.8 1.1 9 .8 1.1 1.7 2.3 2.8 2.3 1.1 0 1.6-.7 2.9-.7 1.4 0 1.8.7 2.9.7 1.2 0 2-1.1 2.7-2.2.9-1.3 1.2-2.5 1.2-2.6 0 0-2.4-.9-2.4-3.6 0-2.2 1.8-3.3 1.9-3.4-1-1.5-2.6-1.7-3.2-1.7z"
		/>
	</svg>
{/snippet}

<!-- hooks.server.ts sets up the reader's page on the server, so links to it need a full page load. -->
{#snippet actions()}
	<div class="actions">
		<a class="download" href={resolve('/download')} data-sveltekit-reload>
			{@render apple()}
			Download for macOS
		</a>
		<a class="web" href={resolve('/read')} data-sveltekit-reload>
			<svg viewBox="0 0 16 16" aria-hidden="true">
				<circle cx="8" cy="8" r="6.5" />
				<path d="M1.5 8h13M8 1.5c-1.8 1.7-2.8 4-2.8 6.5s1 4.8 2.8 6.5M8 1.5c1.8 1.7 2.8 4 2.8 6.5s-1 4.8-2.8 6.5" />
			</svg>
			Read on the web
		</a>
	</div>
{/snippet}

{#snippet soon()}
	<span class="soon">
		<svg viewBox="0 0 12 16" aria-hidden="true">
			<rect x="1.5" y="1" width="9" height="14" rx="2.2" />
			<path d="M4.8 12.5h2.4" />
		</svg>
		iPhone and iPad app coming soon
	</span>
{/snippet}

{#snippet card(item: (typeof reading)[number], i: number)}
	<article class="card" style:--delay="{i * 90}ms" {@attach reveal}>
		<item.Visual paused={paused[item.title]} />
		<div class="caption">
			<div class="heading">
				<h3>{item.title}</h3>
				{#if !reducedMotion.current}
					<button
						type="button"
						class="pause"
						aria-label="{paused[item.title] ? 'Play' : 'Pause'} the {item.title} animation"
						onclick={() => (paused[item.title] = !paused[item.title])}
					>
						{#if paused[item.title]}
							<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.2v9.6L12.6 8z" /></svg>
						{:else}
							<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 3h2.4v10H4.5zM9.1 3h2.4v10H9.1z" /></svg>
						{/if}
					</button>
				{/if}
			</div>
			<p>{item.body}</p>
		</div>
	</article>
{/snippet}

<Nav ontoggle={() => player.sound && play('switch')} />

<main id="top">
	<header class="hero">
		<img class="logo" src="/icon.png" width="112" height="112" alt="" />
		<h1>The Bible, instantly.</h1>
		<p class="lede">
			A very fast, native Bible reader for the Mac, written in Rust.<br />
			Type a reference, press Return, and the passage is already there.
		</p>
		<div class="cta">
			{@render actions()}
			<p class="meta">
				Free · Version {VERSION} · {ARCH} · macOS {MIN_MACOS} or later<DownloadCount repo={REPO_URL} />
			</p>
			{@render soon()}
		</div>
	</header>

	<section class="showcase" aria-label="Demo">
		<Showcase {player} />
	</section>

	<div id="features">
		<section class="section" aria-labelledby="reading-title">
			<h2 id="reading-title" {@attach reveal}>Reading</h2>
			<div class="cards">
				{#each reading as item, i (item.title)}
					{@render card(item, i)}
				{/each}
			</div>
		</section>

		<section class="section" aria-labelledby="study-title">
			<h2 id="study-title" {@attach reveal}>Built for study</h2>
			<div class="cards">
				{#each study as item, i (item.title)}
					{@render card(item, i % 2)}
				{/each}
			</div>
		</section>
	</div>

	<section id="shortcuts" class="section split" aria-labelledby="shortcuts-title">
		<div class="intro" {@attach reveal}>
			<h2 id="shortcuts-title">Shortcuts</h2>
			<p>Every action has a key, and most can be changed in Settings. ⌘/ lists them all.</p>
		</div>
		<ul class="shortcuts" {@attach reveal}>
			{#each shortcuts as shortcut (shortcut.label)}
				<li>
					<span class="keys">
						{#each shortcut.keys as key (key)}
							<kbd class={{ primary: shortcut.primary }}>{key}</kbd>
						{/each}
					</span>
					<span class="label">{shortcut.label}</span>
				</li>
			{/each}
		</ul>
	</section>

	<section id="install" class="section" aria-labelledby="install-title">
		<div class="intro" {@attach reveal}>
			<h2 id="install-title">Install</h2>
			<p>Free. macOS {MIN_MACOS} or later on {ARCH}, or in your browser.</p>
			{@render actions()}
			{@render soon()}
		</div>
	</section>
</main>

<Footer>Scripture on this page is from the KJV, WEB and BSB, which are in the public domain.</Footer>

<style>
	.hero {
		display: flex;
		flex-direction: column;
		align-items: center;
		max-width: 900px;
		margin: 0 auto;
		padding: 72px 24px 48px;
		text-align: center;
	}

	.hero > * {
		animation: rise 0.9s cubic-bezier(0.2, 0.7, 0.2, 1) both;
	}

	.hero > :nth-child(2) {
		animation-delay: 0.08s;
	}

	.hero > :nth-child(3) {
		animation-delay: 0.16s;
	}

	.hero > :nth-child(4) {
		animation-delay: 0.24s;
	}

	.logo {
		display: block;
		filter: drop-shadow(0 18px 40px rgba(0, 0, 0, 0.55)) drop-shadow(0 0 40px rgba(110, 140, 255, 0.14));
		animation-name: pop;
	}

	:global(:root[data-theme='light']) .logo {
		filter: drop-shadow(0 18px 32px rgba(0, 0, 0, 0.2)) drop-shadow(0 0 40px rgba(110, 140, 255, 0.1));
	}

	h1 {
		margin: 32px 0 0;
		font: 700 58px/1.05 var(--display);
		letter-spacing: -0.035em;
		text-wrap: balance;
	}

	.lede {
		max-width: 580px;
		margin: 20px 0 0;
		color: var(--muted);
		font-size: 18px;
		line-height: 29px;
		text-wrap: pretty;
	}

	.cta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		margin-top: 32px;
	}

	.actions {
		--solid-edge: 0 10px 30px rgba(255, 255, 255, 0.07);
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 12px;
	}

	:global(:root[data-theme='light']) .actions {
		--solid-edge: 0 10px 30px rgba(0, 0, 0, 0.14);
	}

	/* The Mac download leads and the web version follows as the raised button. */
	.download {
		--fill: var(--solid);
		--fill-hover: var(--solid-hover);
		--ink: var(--on-solid);
		--edge: var(--solid-edge);
	}

	.web {
		--fill: var(--raised);
		--fill-hover: var(--raised), color-mix(in srgb, var(--fg) 8%, transparent);
		--ink: var(--fg);
		--edge: var(--raised-edge);
	}

	.download,
	.web {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 9px;
		padding: 13px 22px 13px 20px;
		border-radius: 999px;
		background: var(--fill);
		box-shadow: var(--edge);
		color: var(--ink);
		font: 600 15px/18px var(--sans);
		text-decoration: none;
		transition:
			scale 0.2s,
			background-color 0.2s;
	}

	.download:hover,
	.web:hover {
		background: var(--fill-hover);
		scale: 1.03;
	}

	.download svg {
		width: 15px;
		height: 18px;
		fill: currentColor;
	}

	.web svg {
		width: 16px;
		height: 16px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.3;
		stroke-linecap: round;
	}

	/* Phones and tablets can't run the Mac app, so there the web version leads. */
	@media (max-width: 640px), (hover: none) and (pointer: coarse) {
		.web {
			order: -1;
			--fill: var(--solid);
			--fill-hover: var(--solid-hover);
			--ink: var(--on-solid);
			--edge: var(--solid-edge);
		}

		.download {
			--fill: var(--raised);
			--fill-hover: var(--raised), color-mix(in srgb, var(--fg) 8%, transparent);
			--ink: var(--fg);
			--edge: var(--raised-edge);
		}
	}

	.soon {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 5px 12px 5px 10px;
		border-radius: 999px;
		box-shadow: inset 0 0 0 1px var(--line);
		color: var(--muted);
		font-size: 13px;
		line-height: 16px;
	}

	.soon svg {
		width: 10px;
		height: 14px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.3;
		stroke-linecap: round;
	}

	.meta {
		margin: 0;
		color: var(--muted);
		font-size: 13px;
		text-wrap: balance;
	}

	.showcase {
		max-width: 1248px;
		margin: 0 auto;
		padding: 8px 24px 72px;
		animation: rise 1s 0.32s cubic-bezier(0.2, 0.7, 0.2, 1) both;
	}

	.section {
		max-width: 1080px;
		margin: 0 auto;
		padding: 56px 24px;
		border-top: 1px solid var(--line);
	}

	h2 {
		margin: 0 0 24px;
		font: 600 22px/28px var(--display);
		letter-spacing: -0.02em;
	}

	.cards {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 20px;
	}

	.card {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: 16px;
		background: var(--card);
		box-shadow: var(--card-edge);
	}

	.caption {
		padding: 20px 22px 24px;
	}

	.heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	h3 {
		margin: 0;
		font: 600 16px/20px var(--sans);
		letter-spacing: -0.01em;
	}

	.pause {
		display: flex;
		flex-shrink: 0;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		margin: -3px -5px -3px 0;
		border: 0;
		border-radius: 999px;
		background: transparent;
		color: var(--muted);
		cursor: pointer;
		transition:
			background 0.15s,
			color 0.15s;
	}

	.pause:hover {
		background: color-mix(in srgb, var(--fg) 7%, transparent);
		color: var(--fg);
	}

	.pause svg {
		width: 12px;
		height: 12px;
		fill: currentColor;
	}

	.caption p {
		margin: 6px 0 0;
		color: var(--muted);
		font-size: 14px;
		line-height: 22px;
	}

	.split {
		display: grid;
		grid-template-columns: 300px minmax(0, 1fr);
		gap: 40px;
	}

	.intro h2 {
		margin-bottom: 10px;
	}

	.intro p {
		max-width: 260px;
		margin: 0;
		color: var(--muted);
		font-size: 14px;
		line-height: 22px;
	}

	#install .intro p {
		max-width: none;
	}

	.intro .actions {
		justify-content: flex-start;
		margin-top: 22px;
	}

	.intro .soon {
		margin-top: 18px;
	}

	.shortcuts {
		display: grid;
		grid-template-rows: repeat(5, auto);
		grid-auto-flow: column;
		gap: 0 32px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.shortcuts li {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 9px 0;
	}

	.keys {
		display: flex;
		flex-shrink: 0;
		gap: 4px;
		width: 96px;
	}

	kbd {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 24px;
		height: 24px;
		padding: 0 6px;
		border-radius: 6px;
		background: linear-gradient(#2c2c2f, #222225);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.1),
			0 1px 0 #000;
		color: #d1d1d6;
		font: 12px/14px var(--sans);
	}

	kbd.primary {
		background: linear-gradient(#fff, #f2f2f6 55%, #e6e6ec);
		box-shadow:
			inset 0 1px 0 #fff,
			0 1px 0 #000;
		color: #1c1c1e;
		font-weight: 600;
	}

	:global(:root[data-theme='light']) kbd {
		background: linear-gradient(#fff, #f2f2f4);
		box-shadow:
			inset 0 0 0 0.5px rgba(0, 0, 0, 0.12),
			0 1px 0 rgba(0, 0, 0, 0.16);
		color: #3a3a3c;
	}

	:global(:root[data-theme='light']) kbd.primary {
		background: linear-gradient(#48484a, #2c2c2e 55%, #1c1c1e);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.18),
			0 1px 0 rgba(0, 0, 0, 0.3);
		color: #fff;
	}

	.label {
		color: var(--muted);
		font-size: 14px;
		line-height: 18px;
	}

	@keyframes rise {
		from {
			opacity: 0;
			translate: 0 16px;
		}
	}

	@keyframes pop {
		from {
			opacity: 0;
			scale: 0.86;
		}
	}

	@media (max-width: 860px) {
		.cards,
		.split {
			grid-template-columns: minmax(0, 1fr);
		}

		.split {
			gap: 28px;
		}
	}

	@media (max-width: 640px) {
		.hero {
			padding-top: 48px;
		}

		h1 {
			font-size: 40px;
		}

		.lede {
			font-size: 16px;
			line-height: 25px;
		}

		.lede br {
			display: none;
		}

		.actions {
			flex-direction: column;
			width: min(100%, 300px);
		}

		.showcase {
			padding-inline: 16px;
		}

		.section {
			padding-inline: 16px;
		}

		.shortcuts {
			grid-template-rows: none;
			grid-auto-flow: row;
		}
	}
</style>
