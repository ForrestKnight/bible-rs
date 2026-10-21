<!--
	A page with nothing in it yet (bookmarks.rs `empty_page`): the empty state set optically in the
	page below the toolbar, never above the head, with Open John 1 first among its buttons.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { getApp } from '../app.svelte';
	import { Button, EmptyState } from '../ui';
	import type { IconName } from '../ui';

	interface Props {
		glyph: IconName;
		title: string;
		body: string;
		hints?: [string[], string][];
		width?: number;
		/** Buttons after Open John 1. */
		more?: Snippet;
	}

	let { glyph, title, body, hints = [], width = 360, more }: Props = $props();

	const app = getApp();

	const openJohn = () => app.dispatch({ type: 'jump', chapter: app.chapterIndex({ book: 42, chapter: 1, verse: 1 }) });
</script>

<div class="layer">
	<EmptyState {glyph} tone="accent" {title} {body} {hints} {width} minTop={102}>
		{#snippet buttons()}
			<Button kind="primary" label="Open John 1" onclick={openJohn} />
			{@render more?.()}
		{/snippet}
	</EmptyState>
</div>

<style>
	.layer {
		position: absolute;
		top: var(--toolbar);
		right: 40px;
		bottom: 0;
		left: 40px;
	}
</style>
