<!--
	Settings › About (settings_ui.rs `about`): the app, the translations' and fonts' licences.
-->
<script lang="ts">
	import { resolve } from '$app/paths';
	import { REPO_URL } from '#lib/config.js';
	import { AppIcon, Icon } from '../ui';

	const TRANSLATIONS = [
		['KJV', 'Public domain (Crown letters patent in the UK)'],
		['WEB', 'Public domain · “World English Bible” is a trademark of eBible.org'],
		['BSB', 'Dedicated to the public domain, 2023'],
		['ASV · YLT · DBY · WBS', 'Public domain']
	];
	const FONTS = [
		['Literata', 'SIL Open Font License 1.1'],
		['Inter', 'SIL Open Font License 1.1']
	];
</script>

<div class="hero">
	<AppIcon size={88} />
	<h3 class="name">Bible</h3>
	<p class="version">Web reader · works offline</p>
	<p class="tagline">A very fast Bible reader, in the browser and on the Mac.</p>
	<div class="actions">
		<a class="button-link secondary" href={resolve('/download')} data-sveltekit-reload>Get the Mac App</a>
		<a class="button-link ghost" href="{REPO_URL}/releases" target="_blank" rel="noopener">
			Release Notes<Icon name="external" size={16} heavy />
		</a>
	</div>
</div>

<h3 class="group-title">Translations</h3>
<div class="rows">
	{#each TRANSLATIONS as [k, v] (k)}
		<div class="row" style:height="40px"><span class="k">{k}</span><span class="v">{v}</span></div>
	{/each}
</div>
<p class="footnote">Source texts: eBible.org USFM editions, pinned by SHA-256.</p>

<h3 class="group-title">Fonts</h3>
<div class="rows">
	{#each FONTS as [k, v] (k)}
		<div class="row" style:height="40px"><span class="k">{k}</span><span class="v">{v}</span></div>
	{/each}
</div>

<p class="privacy">No accounts or cookies. The site counts page views anonymously, and your notes stay in this browser.</p>

<style>
	.hero {
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 240px;
		padding-top: 8px;
		margin-bottom: 24px;
	}

	.name {
		height: 40px;
		margin-top: 4px;
		color: var(--ink);
		font: 300 32px/40px var(--font-reading);
		font-variation-settings: 'opsz' 48;
		letter-spacing: -0.015em;
	}

	.version {
		margin-top: 4px;
		color: var(--ink-2);
		font: var(--type-ui);
	}

	.tagline {
		margin-top: 4px;
		color: var(--ink-3);
		font: var(--type-ui);
	}

	.actions {
		display: flex;
		gap: 8px;
		margin-top: 12px;
	}

	.button-link {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 32px;
		padding: 0 14px;
		border-radius: 16px;
		font: 500 13px/20px var(--font-ui);
		text-decoration: none;
	}

	.secondary {
		background: var(--raised);
		box-shadow: var(--shadow-control);
		color: var(--ink);
	}

	.secondary:hover {
		background-image: linear-gradient(var(--hover-wash), var(--hover-wash));
	}

	.ghost {
		color: var(--accent);
	}

	.ghost:hover {
		background: var(--fill);
	}

	.k {
		flex: none;
		width: 138px;
		color: var(--ink);
		font: 700 12px/16px var(--font-ui);
	}

	.v {
		color: var(--ink-2);
		font: var(--type-small);
	}

	.privacy {
		margin-top: 24px;
		color: var(--ink-3);
		font: var(--type-small);
		text-align: center;
	}
</style>
