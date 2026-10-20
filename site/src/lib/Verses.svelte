<script lang="ts">
	import type { Run, Verse } from './scripture.js';

	/**
	 * Verses typeset as the reader shows them: paragraphs, superscript numbers, the words of Christ
	 * in red. `tint` gives a verse a background, for highlights, selection and the jump flash.
	 */
	let { verses, tint = {} }: { verses: Verse[]; tint?: Record<number, string | undefined> } = $props();

	let paragraphs = $derived(
		verses.reduce<Verse[][]>((out, v, i) => {
			if (i === 0 || v.paragraph) out.push([]);
			out[out.length - 1].push(v);
			return out;
		}, [])
	);
</script>

{#snippet runs(list: Run[])}
	{#each list as run, i (i)}<span class={{ red: run.red, italic: run.italic, sc: run.smallCaps }}>{run.text}</span>{/each}
{/snippet}

{#each paragraphs as paragraph, p (p)}
	<p>
		{#each paragraph as v (v.n)}
			<span class="verse" data-verse={v.n}
				><sup>{v.n}</sup><span class="text" style:background-color={tint[v.n]}>{@render runs(v.runs)}</span></span
			>
		{/each}
	</p>
{/each}

<style>
	p {
		margin: 0;
	}

	p + p {
		margin-top: 0.75em;
		text-indent: 1.7em;
	}

	/* Svelte drops whitespace between iterations of an each block. */
	.verse::after {
		content: ' ';
	}

	sup {
		padding-right: 0.15em;
		color: var(--verse);
		font: 600 0.55em/1 var(--sans);
		vertical-align: 0.75em;
	}

	.text {
		border-radius: 3px;
		box-decoration-break: clone;
		-webkit-box-decoration-break: clone;
		transition: background-color 0.2s;
	}

	.red {
		color: var(--rubric);
	}

	.italic {
		font-style: italic;
	}

	.sc {
		font-variant-caps: small-caps;
	}
</style>
