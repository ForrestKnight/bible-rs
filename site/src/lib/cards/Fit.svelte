<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Attachment } from 'svelte/attachments';

	/** Lays `children` out at `width`×`height` and scales them to the container's width. */
	let { width, height, children }: { width: number; height: number; children: Snippet } = $props();

	let scale = $state(1);

	const fit: Attachment<HTMLElement> = (node) => {
		const observer = new ResizeObserver(() => (scale = node.clientWidth / width));
		observer.observe(node);
		return () => observer.disconnect();
	};
</script>

<div class="fit" style:aspect-ratio="{width} / {height}" {@attach fit}>
	<div class="inner" style:width="{width}px" style:height="{height}px" style:transform="scale({scale})">
		{@render children()}
	</div>
</div>

<style>
	.fit {
		position: relative;
		width: 100%;
		overflow: hidden;
	}

	.inner {
		position: absolute;
		top: 0;
		left: 0;
		transform-origin: 0 0;
	}
</style>
