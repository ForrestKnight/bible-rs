<script lang="ts">
	import { onMount } from 'svelte';

	let { repo }: { repo: string } = $props();

	let count = $state(0);

	onMount(() => {
		const api = `https://api.github.com/repos${new URL(repo).pathname}/releases?per_page=100`;
		fetch(api)
			.then((res) => (res.ok ? res.json() : []))
			.then((releases: { assets: { name: string; download_count: number }[] }[]) => {
				count = releases
					.flatMap((release) => release.assets)
					// Checksums and update archives would inflate the total.
					.filter((asset) => asset.name.endsWith('.dmg'))
					.reduce((sum, asset) => sum + asset.download_count, 0);
			})
			.catch(() => {});
	});
</script>

{#if count}
	&nbsp;<span>· {count.toLocaleString('en-US')} {count === 1 ? 'download' : 'downloads'}</span>
{/if}

<style>
	span {
		white-space: nowrap;
	}

	@media (prefers-reduced-motion: no-preference) {
		span {
			animation: appear 0.4s ease-out;
		}
	}

	@keyframes appear {
		from {
			opacity: 0;
		}
	}
</style>
