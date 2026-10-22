<!-- A verse's text as the reader sets it: italics, small capitals and the words of Christ in red. -->
<script lang="ts">
	import type { Run } from '../core';
	import { runClass } from '../reader/segments';

	let { runs }: { runs: Run[] } = $props();

	const words = $derived(runs.filter((r) => !r.number));
</script>

{#each words as r, i (i)}{@const c = runClass(r, true)}{#if c}<span class={c}>{r.text}</span>{:else}{r.text}{/if}{/each}

<style>
	.i {
		font-style: italic;
	}

	.b {
		font-weight: 600;
	}

	.sc {
		font-size: 0.75em;
		letter-spacing: 0.04em;
	}

	.red {
		color: var(--rubric);
	}
</style>
