<script lang="ts">
	import { onMount } from 'svelte';

	let { repo }: { repo: string } = $props();

	let count = $state(0);

	onMount(async () => {
		let page: string | undefined =
			`https://api.github.com/repos${new URL(repo).pathname}/releases?per_page=100`;
		let total = 0;
		try {
			while (page) {
				const res = await fetch(page);
				// A partial total would drop every time a release pushes older ones onto the next page.
				if (!res.ok) return;
				const releases: { assets: { name: string; download_count: number }[] }[] =
					await res.json();
				total += releases
					.flatMap((release) => release.assets)
					// Checksums and update archives would inflate the total.
					.filter((asset) => asset.name.endsWith('.dmg'))
					.reduce((sum, asset) => sum + asset.download_count, 0);
				page = res.headers.get('link')?.match(/<([^>]+)>;\s*rel="next"/)?.[1];
			}
		} catch {
			return;
		}
		count = total;
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
