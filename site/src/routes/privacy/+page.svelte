<script lang="ts">
	import { SITE_URL } from '#lib/config.js';
	import ThemeToggle from '#lib/ThemeToggle.svelte';

	const title = 'Privacy policy · Bible';
	const description =
		'Bible collects no data: no account, no analytics, no ads, no tracking. Your highlights, notes and bookmarks stay on your device.';
	const url = `${SITE_URL}/privacy`;
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={url} />
</svelte:head>

<div class="wrap doc">
	<header>
		<ThemeToggle />
		<a href="/"><img class="icon" src="/icon.png" width="64" height="64" alt="Bible home" /></a>
		<h1>Privacy policy</h1>
		<p class="meta">Updated 20 October 2026. Covers Bible for iPhone, iPad and Mac.</p>
	</header>

	<p class="lede">
		Bible collects no data. There's no account, no analytics, no ads and no tracking. Your
		highlights, notes, bookmarks, searches and reading history are never sent to the developer.
	</p>

	<h2>What stays on your device</h2>
	<p>
		The app saves your highlights, notes, bookmarks, reading position, history and settings in a
		file inside it on your iPhone, iPad or Mac. They leave it only when you export or share them.
		On iPhone and iPad, your device's iCloud or computer backups include them, as they do for any
		app. Translations you import on the Mac stay on your Mac.
	</p>

	<h2>When the app uses the internet</h2>
	<p>
		On iPhone and iPad, never. The seven translations are inside the app, and it makes no network
		connections.
	</p>
	<p>The Mac app goes online for two things only:</p>
	<ul>
		<li>
			Checking for updates once a day, which downloads the list of releases from GitHub. The request
			carries only the app's version, and GitHub sees your IP address, as with any download.
		</li>
		<li>
			Online translations you add yourself: the ESV, NLT, NET, or a Bible from API.Bible. The app
			asks that publisher's server directly for the chapters you open, with your API key where one
			is needed. For API.Bible it also reports which passages were shown, with random identifiers,
			as API.Bible's terms require. Each publisher's own privacy policy covers those requests.
		</li>
	</ul>

	<h2>Sharing and crash reports</h2>
	<p>
		When you copy or share a verse, or export your notes, it goes only where you send it. Apple
		passes crash reports to developers only if you've turned on sharing with app developers in your
		device's settings. They describe the crash, not your notes or highlights.
	</p>

	<h2>Children</h2>
	<p>Bible collects nothing from anyone, children included.</p>

	<h2>This website</h2>
	<p>
		This site sets no cookies and runs no analytics. It loads its fonts from Google Fonts and the
		download count from GitHub, which see your IP address when they serve them.
	</p>

	<h2>Changes and questions</h2>
	<p>
		Changes to this policy are published on this page with a new date. Questions are welcome on the
		<a href="/support">support page</a>.
	</p>

	<footer>Made by <a href="https://rasmic.xyz">Micky</a>.</footer>
</div>
