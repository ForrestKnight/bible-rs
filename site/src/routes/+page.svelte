<script lang="ts">
	import { VERSION, SITE_URL } from '#lib/config.js';
	import ThemeToggle from '#lib/ThemeToggle.svelte';

	const title = 'bible-rs: a fast, native Bible for the Mac';
	const description =
		'A very fast, native Bible reader for the Mac. Beautifully typeset, searchable as you type, with highlights, notes and bookmarks kept on your computer.';
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:url" content={SITE_URL} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:image" content={`${SITE_URL}/reader-light.jpg`} />
	<meta property="og:image:width" content="1800" />
	<meta property="og:image:height" content="1209" />
	<meta property="og:image:alt" content="Reading John 1 in bible-rs, light theme" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={`${SITE_URL}/reader-light.jpg`} />
</svelte:head>

<div class="wrap">
	<header>
		<ThemeToggle />
		<img class="icon" src="/icon.png" width="112" height="112" alt="bible-rs app icon" />
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
				alt="Reading John 1 in bible-rs, light theme"
			/>
			<img
				class="dark"
				src="/reader-dark.jpg"
				width="1800"
				height="1209"
				alt="Reading in bible-rs, dark theme"
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
