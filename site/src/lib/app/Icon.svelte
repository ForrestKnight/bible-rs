<script lang="ts" module>
	const PATHS = {
		search: 'M7 2.5a4.5 4.5 0 1 1 0 9a4.5 4.5 0 1 1 0-9zM10.4 10.4 14 14',
		chapter: 'M6 4h7M6 8h7M6 12h7M3 4h.5M3 8h.5M3 12h.5',
		verse: 'M4.5 2.5h7a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1zM6.5 2.5v11',
		parallel: 'M3.5 3h9A1.5 1.5 0 0 1 14 4.5v7a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5v-7A1.5 1.5 0 0 1 3.5 3zM8 3v10',
		sidebar: 'M3.5 3h9A1.5 1.5 0 0 1 14 4.5v7a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5v-7A1.5 1.5 0 0 1 3.5 3zM6 3v10',
		note: 'M4.5 2h7A1.5 1.5 0 0 1 13 3.5v9a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 3 12.5v-9A1.5 1.5 0 0 1 4.5 2zM5.5 6h5M5.5 9h5',
		bookmark: 'M4.5 2.5h7v11L8 11l-3.5 2.5z',
		copy: 'M6.5 5h6A1.5 1.5 0 0 1 14 6.5v6a1.5 1.5 0 0 1-1.5 1.5h-6A1.5 1.5 0 0 1 5 12.5v-6A1.5 1.5 0 0 1 6.5 5zM2.5 10.5V4A1.5 1.5 0 0 1 4 2.5h6.5',
		quote: 'M3.5 9.5c0-3 1-4.5 3-5M9.5 9.5c0-3 1-4.5 3-5M5 9a1.5 1.5 0 1 1 0 3a1.5 1.5 0 1 1 0-3zM11 9a1.5 1.5 0 1 1 0 3a1.5 1.5 0 1 1 0-3z',
		share: 'M8 2v8M5 5l3-3 3 3M4 8.5v4A1.5 1.5 0 0 0 5.5 14h5a1.5 1.5 0 0 0 1.5-1.5v-4',
		compare: 'M3 5.5h10M10.5 3 13 5.5 10.5 8M13 10.5H3M5.5 8 3 10.5 5.5 13',
		range: 'M8 3v10M6 3h4M6 13h4',
		close: 'M4.5 4.5l7 7M11.5 4.5l-7 7',
		clear: 'M8 2a6 6 0 1 1 0 12A6 6 0 1 1 8 2zM6 6l4 4M10 6l-4 4',
		none: 'M8 2a6 6 0 1 1 0 12A6 6 0 1 1 8 2zM3.8 12.2l8.4-8.4',
		trash: 'M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.7 8.5a1 1 0 0 0 1 .9h3.6a1 1 0 0 0 1-.9l.7-8.5',
		back: 'M10 3.5 5.5 8l4.5 4.5',
		forward: 'M6 3.5 10.5 8 6 12.5',
		down: 'M4.5 6.5 8 10l3.5-3.5',
		check: 'M3.5 8.5l3 3 6-7'
	};

	export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
	let { name, size = 14 }: { name: IconName; size?: number } = $props();
</script>

<svg viewBox="0 0 16 16" width={size} height={size} aria-hidden="true"><path d={PATHS[name]} /></svg>

<style>
	svg {
		flex-shrink: 0;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.4;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
