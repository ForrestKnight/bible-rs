<!--
	Links for the chapter being read, which the window otherwise only reaches by keys and menus: the
	trail up to its book and translation, the chapters either side, and the chapter in the other
	translations. They're for screen readers and search engines, so they stay out of sight until one
	has keyboard focus, then show as a bar along the window's foot. The reader follows the chapter
	links itself (ui/follow.ts); the trail goes to the book's and the translation's pages, which an
	imported or a live translation doesn't have.
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { bookOf, bookPath, chapterName, translationPath } from '../books';
	import { CHAPTERS } from '../core';
	import { follow } from '../ui/follow';

	const app = getApp();
	const here = $derived(app.here.chapter);
	const id = $derived(app.settings.translation);
	const book = $derived(bookOf(app.books, here));
	const others = $derived(app.translations.filter((t) => !t.imported && t.id !== id));
	const paged = $derived(!app.info(id)?.imported && !app.isLive(id));
	const name = (index: number) => chapterName(app.books, index);
</script>

<nav class="links" aria-label="Chapter navigation">
	<ol class="trail">
		<li><a href="/" data-sveltekit-reload>Home</a></li>
		{#if paged}
			<li><a href={translationPath(id)} data-sveltekit-reload>{app.info(id)?.abbrev ?? id}</a></li>
			<li><a href={bookPath(id, book)} data-sveltekit-reload>{book.name}</a></li>
		{/if}
		<li aria-current="page">{name(here)}</li>
	</ol>
	<ul>
		{#if here > 0}
			<li>
				<a
					class="follow"
					href={app.chapterHref(here - 1)}
					rel="prev"
					use:follow={() => app.dispatch({ type: 'jump', chapter: here - 1 })}>Previous chapter: {name(here - 1)}</a
				>
			</li>
		{/if}
		{#if here < CHAPTERS - 1}
			<li>
				<a
					class="follow"
					href={app.chapterHref(here + 1)}
					rel="next"
					use:follow={() => app.dispatch({ type: 'jump', chapter: here + 1 })}>Next chapter: {name(here + 1)}</a
				>
			</li>
		{/if}
	</ul>
	{#if others.length}
		<p>{name(here)} in other translations:</p>
		<ul>
			{#each others as t (t.id)}
				<li>
					<a
						class="follow"
						href={app.chapterHref(here, t.id)}
						title={t.name}
						use:follow={() => app.dispatch({ type: 'setTranslation', id: t.id })}>{t.abbrev}</a
					>
				</li>
			{/each}
		</ul>
	{/if}
</nav>

<style>
	/* Out of sight (but read aloud) until a link in it has focus. */
	.links:not(:focus-within) {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	.links:focus-within {
		position: absolute;
		left: 50%;
		bottom: 16px;
		transform: translateX(-50%);
		z-index: 5;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px 16px;
		width: max-content;
		max-width: calc(100% - 32px);
		padding: 8px 16px;
		border-radius: var(--radius-lg);
		background: var(--raised);
		box-shadow: var(--shadow-popover);
		color: var(--ink-2);
		font: var(--type-ui);
	}

	ol,
	ul {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 12px;
	}

	.trail li + li::before {
		content: '›';
		margin-right: 12px;
		color: var(--ink-3);
	}

	a {
		color: var(--accent);
		border-radius: var(--radius-xs);
	}
</style>
