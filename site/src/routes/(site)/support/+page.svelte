<script lang="ts">
	import { resolve } from '$app/paths';
	import { SITE_NAME, SITE_URL } from '#lib/config.js';
	import Doc from '#lib/Doc.svelte';

	const title = `Support | ${SITE_NAME}`;
	const description = 'Help with Bible for iPhone, iPad and Mac, and how to report a problem.';
	const url = `${SITE_URL}/support`;
	const issues = 'https://github.com/michaelshimeles/bible-rs/issues';
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={url} />
</svelte:head>

<Doc heading="Support" meta="Bible for iPhone, iPad and Mac">
	<p class="lede">
		Found a problem or have a question? <a href="{issues}/new">Open an issue on GitHub</a>, or
		<a href={issues}>search the existing ones</a>. Include your device, its system version and the
		app's version. On iPhone and iPad the version is at the bottom of the Text and theme settings;
		on the Mac it's under Bible, then About Bible.
	</p>

	<h2>Back up or move your notes</h2>
	<p>
		On iPhone and iPad, tap the Text and theme button, the one with two letter A's, and use
		<strong>Export as Markdown</strong> or <strong>Export as JSON</strong> under Notes and
		highlights. Your device's iCloud or computer backups also include them.
	</p>

	<h2>Make the text bigger or smaller</h2>
	<p>
		Use the text size control under Text and theme. The app also follows the text size set for
		your whole device.
	</p>

	<h2>Translations</h2>
	<p>
		The KJV, WEB, BSB, ASV, YLT, Darby and Webster translations are built in and work offline. They
		are all in the public domain.
	</p>

	<h2>Privacy</h2>
	<p>
		Bible collects no data, and the iPhone and iPad app never goes online. The
		<a href={resolve('/(site)/privacy')}>privacy policy</a> has the details.
	</p>

</Doc>
