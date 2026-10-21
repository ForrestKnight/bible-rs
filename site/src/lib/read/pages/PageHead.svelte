<!--
	A page's head (pages/mod.rs `title`): Literata 28/36 and a 13/20 line under it, with the page's
	controls at the right on the subtitle's line; the page's body starts 20 pt below.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, subtitle, actions }: { title: string; subtitle: string; actions?: Snippet } = $props();
</script>

<header class="head">
	<div class="titles">
		<h1 class="title">{title}</h1>
		<p class="subtitle">{subtitle}</p>
	</div>
	{#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
	.head {
		display: flex;
		align-items: flex-end;
		gap: 8px;
		height: 58px;
		margin-bottom: 20px;
	}

	.titles {
		flex: 1;
		min-width: 0;
	}

	.title {
		color: var(--ink);
		font: 400 28px/36px var(--font-reading);
		font-variation-settings: 'opsz' 28;
		letter-spacing: -0.01em;
	}

	.subtitle {
		margin-top: 2px;
		color: var(--ink-2);
		font: var(--type-ui);
		white-space: nowrap;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}
</style>
