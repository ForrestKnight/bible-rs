<!-- Links as a row of capsules that wraps: the topics that list a verse, a topic's related topics. -->
<script lang="ts">
	let { links }: { links: { name: string; href: string }[] } = $props();
</script>

<ul>
	{#each links as l (l.href)}
		<li><a href={l.href}>{l.name}</a></li>
	{/each}
</ul>

<style>
	ul {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	a {
		display: inline-flex;
		align-items: center;
		height: 32px;
		padding: 0 14px;
		border-radius: 16px;
		background: var(--fill);
		font: var(--type-ui-strong);
		transition:
			background-color var(--motion-fast),
			color var(--motion-fast);
	}

	a:hover {
		background: var(--fill-strong);
		color: var(--accent);
	}
</style>
