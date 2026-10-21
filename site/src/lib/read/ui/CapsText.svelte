<!--
	Plain verse text with its all-capital words ("LORD") set as small capitals: the first letter at
	full size, the rest at 0.75 em (widgets.rs `caps_words`).
-->
<script lang="ts">
	let { text }: { text: string } = $props();

	const CAPS = /(?<!\p{L})(\p{Lu})(\p{Lu}+)(?!\p{L})/gu;

	const parts = $derived.by(() => {
		const out: { text: string; caps?: string }[] = [];
		let at = 0;
		for (const m of text.matchAll(CAPS)) {
			out.push({ text: text.slice(at, m.index) + m[1], caps: m[2] });
			at = m.index + m[0].length;
		}
		out.push({ text: text.slice(at) });
		return out;
	});
</script>

{#each parts as p, i (i)}{p.text}{#if p.caps}<span class="sc">{p.caps}</span>{/if}{/each}

<style>
	/* No line box of its own, which would make the line taller than the text's. */
	.sc {
		font-size: 0.75em;
		line-height: 0;
		letter-spacing: 0.04em;
	}
</style>
