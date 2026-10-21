<!--
	Share as image (share_sheet.rs): the verse on a card, its style, shape and what it shows, and
	how its text is copied. The image is the card drawn again on a canvas from the same layout.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { getApp } from '../app.svelte';
	import { rangeLabel, type VerseRef } from '../core';
	import type { CardShape, CardStyle, ShareCopy } from '../settings';
	import { CARD_FONT, VERSE_DROP, cardColours, cardFonts, cardImage, layoutCard, previewText, type Seg } from '../share';
	import { Button, Icon, Segmented, Sheet, Toggle } from '../ui';

	const app = getApp();
	const o = $derived(app.settings.share);

	const STYLES: { value: CardStyle; label: string; theme: 'light' | 'sepia' | 'dark' }[] = [
		{ value: 'paper', label: 'Paper', theme: 'light' },
		{ value: 'sepia', label: 'Sepia', theme: 'sepia' },
		{ value: 'night', label: 'Night', theme: 'dark' },
		{ value: 'ink', label: 'Ink', theme: 'light' }
	];
	const SHAPES: { value: CardShape; label: string }[] = [
		{ value: 'square', label: 'Square' },
		{ value: 'portrait', label: 'Portrait' },
		{ value: 'wide', label: 'Wide' }
	];
	const COPIES: { value: ShareCopy; label: string }[] = [
		{ value: 'plain', label: 'Plain' },
		{ value: 'reference', label: 'With reference' },
		{ value: 'markdown', label: 'Markdown' }
	];
	/** The options column less the preview box's sides. */
	const PREVIEW_WIDTH = 336 - 24;

	const refs = $derived.by(() => {
		const sorted = [...(app.share ?? [])].sort((a, b) => a.book - b.book || a.chapter - b.chapter || a.verse - b.verse);
		return sorted.filter((r, i) => i === 0 || !same(r, sorted[i - 1]));
	});
	const reference = $derived(refs.length ? rangeLabel(refs[0], refs[refs.length - 1]) : '');
	const id = $derived(app.settings.translation);
	const name = $derived(app.info(id)?.name ?? '');

	const runs = $derived.by(() => {
		const out: Seg[] = [];
		for (const r of refs) {
			const v = app.core?.verse(id, r);
			if (!v) continue;
			if (out.length) out.push({ text: ' ', red: false });
			for (const run of v.runs) if (!run.number) out.push({ text: run.text, red: o.redLetters && run.red });
		}
		return out;
	});

	let fontsReady = $state(false);
	onMount(() => void cardFonts().then(() => (fontsReady = true)));
	const layout = $derived(fontsReady ? layoutCard(o.shape, runs, o.reference) : null);
	const theme = $derived(STYLES.find((s) => s.value === o.style)!.theme);

	const copy = $derived(app.core?.copyText(id, refs, o.copy) ?? '');
	const shown = $derived(fontsReady ? previewText(copy, PREVIEW_WIDTH) : copy);

	let card = $state<HTMLElement>();

	function same(a: VerseRef, b: VerseRef) {
		return a.book === b.book && a.chapter === b.chapter && a.verse === b.verse;
	}

	const close = () => (app.share = null);
	const fileName = () => `${reference.replace(/:/g, '.')}.png`;

	async function image(): Promise<Blob | null> {
		if (!layout || !card) return null;
		try {
			return await cardImage(layout, cardColours(o.style, card), {
				reference: o.reference ? reference : null,
				translation: o.translation ? name : null
			});
		} catch {
			app.toastNow("Couldn't make the image");
			return null;
		}
	}

	function download(blob: Blob) {
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = fileName();
		a.click();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
		app.toastNow(`Saved ${fileName()}`);
	}

	async function save() {
		const blob = await image();
		if (blob) download(blob);
	}

	async function share() {
		const blob = await image();
		if (!blob) return;
		const file = new File([blob], fileName(), { type: 'image/png' });
		if (navigator.canShare?.({ files: [file] })) {
			try {
				await navigator.share({ files: [file], title: reference });
			} catch (e) {
				if ((e as Error).name !== 'AbortError') download(blob);
			}
		} else {
			download(blob);
		}
	}

	async function copyText() {
		try {
			await navigator.clipboard.writeText(copy);
			app.toastNow('Copied');
		} catch {
			app.toastNow("Couldn't copy");
		}
	}
</script>

<Sheet label="Share {reference}" onclose={close} width={760} top={72} focusSheet>
	<header class="head">
		<h2 class="title">Share {reference}</h2>
	</header>
	<div class="body">
		<div class="well">
			<div
				bind:this={card}
				class={['card', o.style]}
				data-theme-preview={theme}
				style:width="{layout?.width ?? 300}px"
				style:height="{layout?.height ?? 376}px"
				style:padding="{layout?.pad ?? 32}px"
			>
				{#if o.reference}<p class="label">{reference}</p>{/if}
				{#if layout}
					<p
						class="verse"
						style:font-family={CARD_FONT}
						style:top="{VERSE_DROP}px"
						style:font-size="{layout.size}px"
						style:line-height="{layout.lineHeight}px"
					>
						{#each layout.lines as line, i (i)}
							<span class="line">{#each line as seg, j (j)}<span class={{ red: seg.red }}>{seg.text}</span>{/each}</span>
						{/each}
					</p>
				{/if}
				<footer class="card-foot">
					{#if o.translation}<span class="name">{name}</span>{/if}
					<span class="brand"><Icon name="book" size={12} />Bible</span>
				</footer>
			</div>
			{#if layout?.cut}<p class="cut">Too long for one card: it ends with “…”</p>{/if}
		</div>

		<div class="options">
			<h3 class="caption">Style</h3>
			<div class="styles" role="radiogroup" aria-label="Style">
				{#each STYLES as s (s.value)}
					<button
						type="button"
						class={['style', { on: o.style === s.value }]}
						role="radio"
						aria-checked={o.style === s.value}
						onclick={() => (o.style = s.value)}
					>
						<span class="frame">
							<span class={['tile', s.value]} data-theme-preview={s.theme}>
								<span class="bar" style:width="40px"></span>
								<span class="bar" style:width="52px"></span>
								<span class="bar" style:width="32px"></span>
							</span>
						</span>
						<span class="style-label">{s.label}</span>
					</button>
				{/each}
			</div>

			<h3 class="caption">Shape</h3>
			<Segmented options={SHAPES} bind:value={o.shape} layout="tight" label="Shape" />

			<div class="toggles">
				<div class="toggle"><span>Include reference</span><Toggle bind:checked={o.reference} label="Include reference" /></div>
				<div class="toggle"><span>Include translation</span><Toggle bind:checked={o.translation} label="Include translation" /></div>
				<div class="toggle"><span>Red letters</span><Toggle bind:checked={o.redLetters} label="Red letters" /></div>
			</div>

			<h3 class="caption">Copy as text</h3>
			<Segmented options={COPIES} bind:value={o.copy} layout="tight" label="Copy as text" />
			<p class="preview">{shown}</p>
		</div>
	</div>
	<footer class="foot">
		<Button kind="plain" icon="copy" label="Copy Text" onclick={copyText} />
		<span class="spacer"></span>
		<Button kind="secondary" label="Save Image…" onclick={save} />
		<Button kind="primary" icon="share" label="Share…" onclick={share} />
	</footer>
</Sheet>

<style>
	.head {
		display: flex;
		align-items: center;
		height: 52px;
		padding: 0 24px;
		box-shadow: inset 0 -1px var(--separator);
	}

	.title {
		font: 600 15px/20px var(--font-ui);
		color: var(--ink);
	}

	.body {
		display: flex;
		gap: 24px;
		height: 488px;
		padding: 24px;
	}

	.well {
		display: grid;
		position: relative;
		place-items: center;
		flex: none;
		width: 352px;
		height: 440px;
		border-radius: 14px;
		background: var(--fill);
	}

	.cut {
		position: absolute;
		right: 0;
		bottom: 12px;
		left: 0;
		color: var(--ink-3);
		font: var(--type-small);
		text-align: center;
	}

	.card {
		display: flex;
		flex-direction: column;
		border-radius: 16px;
		background: var(--paper);
		box-shadow:
			0 0 0 0.5px rgb(0 0 0 / 0.1),
			0 12px 64px rgb(0 0 0 / 0.18);
		color: var(--ink);
		--card-label: var(--accent);
		--card-footer: var(--ink-2);
		--card-brand: var(--ink-3);
		--card-rubric: var(--rubric);
	}

	.card.ink {
		background: var(--accent);
		color: #fff;
		--card-label: #fff;
		--card-footer: rgb(255 255 255 / 0.784);
		--card-brand: rgb(255 255 255 / 0.627);
		--card-rubric: rgb(255 196 186);
	}

	.label {
		margin-bottom: 16px;
		color: var(--card-label);
		font: 600 11px/16px var(--font-ui);
		letter-spacing: 0.12em;
		text-transform: uppercase;
		white-space: nowrap;
	}

	.verse {
		position: relative;
		flex: 1;
		min-height: 0;
		font-weight: 400;
		white-space: pre;
	}

	.line {
		display: block;
	}

	.red {
		color: var(--card-rubric);
	}

	.card-foot {
		display: flex;
		align-items: center;
		height: 16px;
		margin-top: auto;
		font: 500 11px/16px var(--font-ui);
		color: var(--card-footer);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-left: auto;
		color: var(--card-brand);
		font-weight: 600;
	}

	.options {
		flex: 1;
		min-width: 0;
	}

	.caption {
		margin-bottom: 8px;
		color: var(--ink-2);
		font: 600 12px/16px var(--font-ui);
	}

	.styles {
		display: flex;
		gap: 14px;
		margin-bottom: 18px;
	}

	.style {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		width: 72px;
	}

	.frame {
		border-radius: 10px;
		box-shadow: 0 0 0 0.5px var(--border);
	}

	.style:hover:not(.on) .frame {
		box-shadow: 0 0 0 0.5px var(--ink-3);
	}

	.style.on .frame {
		box-shadow:
			0 0 0 2px var(--accent),
			0 0 0 5px var(--focus);
	}

	.tile {
		display: flex;
		flex-direction: column;
		gap: 5px;
		width: 72px;
		height: 56px;
		padding: 12px;
		border-radius: 10px;
		background: var(--paper);
		--bar: var(--ink);
	}

	.tile.ink {
		background: var(--accent);
		--bar: #fff;
	}

	.bar {
		flex: none;
		height: 3px;
		border-radius: 2px;
		background: color-mix(in srgb, var(--bar) 60%, transparent);
	}

	.style-label {
		color: var(--ink-2);
		font: 500 12px/16px var(--font-ui);
	}

	.style.on .style-label {
		color: var(--accent);
		font-weight: 600;
	}

	.toggles {
		display: flex;
		flex-direction: column;
		gap: 10px;
		margin: 18px 0 18px;
	}

	.toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 24px;
		color: var(--ink);
		font: var(--type-ui);
	}

	.preview {
		display: -webkit-box;
		height: 52px;
		margin-top: 8px;
		padding: 9px 12px 11px;
		overflow: hidden;
		border-radius: 8px;
		background: var(--fill);
		color: var(--ink-2);
		font: 12px/16px ui-monospace, SFMono-Regular, Menlo, monospace;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		line-clamp: 2;
	}

	.foot {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 64px;
		padding: 0 24px;
		border-radius: 0 0 16px 16px;
		background: var(--fill);
		box-shadow: inset 0 1px var(--separator);
	}

	.spacer {
		flex: 1;
	}
</style>
