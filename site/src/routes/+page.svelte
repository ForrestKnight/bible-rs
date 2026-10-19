<script lang="ts">
	import { VERSION, SITE_URL } from '#lib/config.js';
	import ThemeToggle from '#lib/ThemeToggle.svelte';

	const name = 'Bible';
	const title = `${name}: a fast, native reader for the Mac`;
	const description =
		'A very fast, native Bible reader for the Mac. Beautifully typeset, searchable as you type, with highlights, notes and bookmarks kept on your computer.';
	const url = `${SITE_URL}/`;
	const image = `${SITE_URL}/og.jpg`;
	const imageAlt = `The ${name} app icon and name beside the reader open at John 1`;

	const jsonLd = JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'SoftwareApplication',
		name,
		description,
		url,
		image,
		operatingSystem: 'macOS',
		applicationCategory: 'ReferenceApplication',
		offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
		downloadUrl: `${SITE_URL}/download`,
		softwareVersion: VERSION
	}).replace(/</g, '\\u003c');
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={url} />
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content={name} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={url} />
	<meta property="og:locale" content="en_US" />
	<meta property="og:image" content={image} />
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />
	<meta property="og:image:type" content="image/jpeg" />
	<meta property="og:image:alt" content={imageAlt} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={image} />
	<meta name="twitter:image:alt" content={imageAlt} />
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- static JSON, with < escaped -->
	{@html `<script type="application/ld+json">${jsonLd}<\/script>`}
</svelte:head>

<div class="wrap">
	<header>
		<ThemeToggle />
		<img class="icon" src="/icon.png" width="112" height="112" alt="Bible app icon" />
		<h1>The Bible, instantly.</h1>
		<p class="lede">{description}</p>
		<a class="btn" id="download" href="/download" data-sveltekit-reload>
			<svg
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"><path d="M12 3v12M6 11l6 6 6-6M5 21h14" /></svg
			>
			Download for Mac
		</a>
		<div class="meta">Version {VERSION} · Apple Silicon and Intel · Free</div>
		<div class="shot">
			<img
				class="light"
				src="/reader-light.jpg"
				width="1800"
				height="1209"
				alt="John 1 open in Bible, light theme"
			/>
			<img
				class="dark"
				src="/reader-dark.jpg"
				width="1800"
				height="1209"
				alt="Matthew 5 open in Bible, dark theme"
			/>
		</div>
	</header>
	<section class="features">
		<div class="f">
			<h3>Instant</h3>
			<p>Opens straight to the text. Search the whole Bible as you type, in under 2 ms.</p>
		</div>
		<div class="f">
			<h3>Typeset for reading</h3>
			<p>
				Literata serif, a comfortable measure, poetry with hanging indents, and light, sepia and
				dark themes.
			</p>
		</div>
		<div class="f">
			<h3>Many translations</h3>
			<p>
				KJV, WEB, BSB, ASV, YLT, Darby and Webster built in, side by side. Add the NLT and NET free
				in one click.
			</p>
		</div>
		<div class="f">
			<h3>Keyboard first</h3>
			<p>
				<kbd>⌘K</kbd> jumps anywhere, <kbd>←</kbd> <kbd>→</kbd> turn chapters, <kbd>⌘F</kbd> searches,
				<kbd>1</kbd>–<kbd>6</kbd> highlight.
			</p>
		</div>
		<div class="f">
			<h3>Yours</h3>
			<p>Highlights, notes and bookmarks stay in a local file on your Mac.</p>
		</div>
		<div class="f">
			<h3>Native</h3>
			<p>Written in Rust. Small, quiet on battery, and smooth on 120 Hz displays.</p>
		</div>
	</section>
	<footer>
		Requires macOS on Apple Silicon or Intel. Made by <a href="https://rasmic.xyz">Micky</a>.
	</footer>
</div>
