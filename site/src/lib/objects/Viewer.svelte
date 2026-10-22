<!--
	The interactive Ark: the 3D scene (scene.ts, loaded after the page paints and only where WebGL
	works), its controls, numbered hotspots for every part, and a panel with the verses behind the
	part you tap, in the reader's current translation. Without WebGL it shows a still render and the
	same parts list. The Ark page and its embed (the Mac app's webview) both use it.
-->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { Badge, Button, Icon, Segmented, SelectField, Toggle } from '../read/ui';
	import type { ArkScene, Hotspot } from './scene';
	import { citedRefs, cubitsLabel, inUnits, partRefs, refLabel, verseLink, type Cubits, type ObjectSpec, type VerseTexts } from './spec';
	import { available, inApp, readerTranslation, textsIn, type ImportedTexts } from './texts';

	interface Props {
		spec: ObjectSpec;
		cubits: Cubits;
		/** The KJV's text of the cited verses, prerendered with the page. */
		kjv: VerseTexts;
		/** Where the other translations' text is (verses.json). */
		versesUrl: string;
		/** The still render, shown until the scene draws and in its place without WebGL. */
		poster: string;
		/** The Mac app's webview: the viewer fills the window and verse links open the app's reader. */
		embed?: boolean;
		/** Reads a translation imported into this browser's reader, which verses.json can't have. */
		imported?: ImportedTexts;
	}

	let { spec, cubits, kjv, versesUrl, poster, embed = false, imported }: Props = $props();

	const refs = $derived(citedRefs(spec, cubits));
	const assembly = $derived(spec.assembly);

	let stage: HTMLDivElement | undefined = $state();
	let scene = $state.raw<ArkScene | null>(null);
	let status = $state<'loading' | 'ready' | 'still'>('loading');
	let reducedMotion = $state(false);
	/** ?shot: the model alone, filling the window, for the still and share images (scripts/ark-og.sh). */
	let shot = $state(false);
	/** ?shot=og: without the person, framed closer. */
	let og = false;

	// The props are the page's data, which doesn't change; these start from them.
	let cubitId = $state(untrack(() => cubits.default));
	let person = $state(true);
	let inside = $state(false);
	let closeUp = $state(false);
	/** Each debated option's choice, by "part.option": its default to start with. */
	let choices = $state<Record<string, string>>(
		untrack(() =>
			Object.fromEntries(spec.parts.flatMap((p) => (p.options ?? []).map((o) => [`${p.id}.${o.id}`, o.default])))
		)
	);
	let selected = $state<string | null>(null);
	let step = $state<number | null>(null);
	let building = $state(false);
	let spots = $state<Hotspot[]>([]);

	let texts = $state<VerseTexts>(untrack(() => kjv));
	let translations = $state<{ id: string; abbrev: string; name: string }[]>([]);
	/** The translations verses.json has, whose verses have pages on the site to link to. */
	let listed = $state<string[]>(['kjv']);
	/** The translation asked for when it couldn't be shown (the KJV is shown instead). */
	let missing = $state<string | null>(null);

	const cubit = $derived(cubits.cubits.find((c) => c.id === cubitId) ?? cubits.cubits[0]);
	const part = $derived(spec.parts.find((p) => p.id === selected) ?? null);
	const chest = $derived(spec.parts.find((p) => p.id === 'chest')!);
	const caption = $derived(step === null ? null : assembly[step]);

	const verse = (ref: string) => texts.verses[ref] ?? kjv.verses[ref] ?? '';
	/** Whose words a verse is in: the KJV's when the chosen translation lacks it. */
	const abbrevOf = (ref: string) => (texts.verses[ref] ? texts.abbrev : kjv.abbrev);
	const linkFor = (ref: string) => verseLink(ref, texts.verses[ref] && listed.includes(texts.id) ? texts.id : 'kjv');

	/** The latest translation asked for: an older request that answers late is ignored. */
	let asked = '';

	async function useTranslation(id: string) {
		asked = id;
		if (id === texts.id) {
			missing = null;
			return;
		}
		const t = id === 'kjv' ? kjv : await textsIn(id, versesUrl, refs, imported);
		if (asked !== id) return;
		if (t) {
			texts = t;
			missing = null;
			if (!translations.some((x) => x.id === t.id)) translations = [...translations, { id: t.id, abbrev: t.abbrev, name: t.name }];
		} else {
			texts = kjv;
			missing = id;
		}
	}

	function pick(id: string | null) {
		selected = id === selected ? null : id;
		if (selected === 'cherubim') closeUp = true;
		if (selected === 'testimony') inside = true;
	}

	function build() {
		if (!scene) return;
		if (scene.building) {
			scene.finish();
			return;
		}
		selected = null;
		closeUp = false;
		scene.build();
		building = true;
	}

	/** In the app, a verse opens the app's own reader instead of a web page. */
	function read(e: MouseEvent, ref: string) {
		if (!inApp()) return;
		e.preventDefault();
		(window as unknown as { ipc: { postMessage(s: string): void } }).ipc.postMessage(JSON.stringify({ type: 'read', ref }));
	}

	function keydown(e: KeyboardEvent) {
		if (e.key !== 'Escape' || e.defaultPrevented) return;
		if (selected) pick(null);
		else if (closeUp) closeUp = false;
		else if (embed && inApp()) (window as unknown as { ipc: { postMessage(s: string): void } }).ipc.postMessage(JSON.stringify({ type: 'close' }));
	}

	function webglWorks(): boolean {
		try {
			const c = document.createElement('canvas');
			return !!(c.getContext('webgl2') ?? c.getContext('webgl'));
		} catch {
			return false;
		}
	}

	onMount(() => {
		reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
		const params = new URLSearchParams(location.search);
		shot = params.has('shot');
		og = params.get('shot') === 'og';
		if (og) person = false;
		// The app names its translation; a browser uses the reader's.
		const wanted = params.get('translation') ?? readerTranslation() ?? 'kjv';
		void available(versesUrl, wanted).then((list) => {
			translations = list.map((t) => ({ id: t.id, abbrev: t.abbrev, name: t.name }));
			if (!inApp()) listed = list.map((t) => t.id);
			if (!translations.some((t) => t.id === 'kjv')) translations = [{ id: kjv.id, abbrev: kjv.abbrev, name: kjv.name }, ...translations];
			return useTranslation(wanted);
		});

		// The app sets the theme and the translation as they change in it.
		(window as unknown as { ark: object }).ark = {
			setTheme: (theme: string) => {
				if (['light', 'sepia', 'dark'].includes(theme)) document.documentElement.dataset.theme = theme;
			},
			setTranslation: (id: string) => {
				void useTranslation(id);
			}
		};

		let cancelled = false;
		let autoplay: IntersectionObserver | null = null;
		if (!webglWorks()) {
			status = 'still';
			return;
		}
		// After the first paint, so the page's text never waits for three.js.
		const start = () =>
			import('./scene').then(({ createScene }) => {
				if (cancelled || !stage) return;
				try {
					scene = createScene(stage, spec, {
						reducedMotion,
						person: cubits.person.inches,
						cubit: cubit.inches,
						touchScroll: !embed,
						tight: og,
						events: {
							pick,
							step: (i) => {
								step = i;
								building = i !== null;
							},
							ready: () => {
								status = 'ready';
								if (inApp()) (window as unknown as { ipc: { postMessage(s: string): void } }).ipc.postMessage(JSON.stringify({ type: 'ready' }));
							}
						}
					});
				} catch {
					status = 'still';
					return;
				}
				scene.onHotspots(() => (spots = scene?.hotspots() ?? []));
				if (shot) return;
				// "Build from the text" plays once, the first time the model is in view.
				if (!reducedMotion)
					autoplay = new IntersectionObserver(
						(entries) => {
							if (!entries.at(-1)?.isIntersecting) return;
							autoplay?.disconnect();
							// Not over something the reader already started or chose.
							if (!scene?.building && !selected && !closeUp && !inside) build();
						},
						{ threshold: 0.6 }
					);
				autoplay?.observe(stage);
			});
		const idle = window.requestIdleCallback ?? ((fn: () => void) => setTimeout(fn, 1));
		idle(() => void start().catch(() => (status = 'still')), { timeout: 600 });
		return () => {
			cancelled = true;
			autoplay?.disconnect();
			scene?.dispose();
			scene = null;
		};
	});

	$effect(() => {
		const inches = cubit.inches;
		scene?.setCubit(inches);
	});
	$effect(() => {
		const on = person;
		scene?.setPerson(on);
	});
	// Also what was chosen before the scene loaded, once it has.
	$effect(() => {
		const on = inside;
		scene?.setInside(on);
	});
	$effect(() => {
		const on = closeUp;
		scene?.setCloseUp(on);
	});
	$effect(() => {
		const id = selected;
		scene?.select(id);
	});
	$effect(() => {
		for (const [key, choice] of Object.entries(choices)) {
			const [id, option] = key.split('.');
			scene?.setChoice(id, option, choice);
		}
	});

	const partNumber = (id: string) => spec.parts.findIndex((p) => p.id === id) + 1;
	const spotFor = (id: string) => spots.find((s) => s.part === id)?.at ?? null;
	const len = (n: number) => inUnits(n, cubit);
</script>

<svelte:window onkeydown={keydown} />

<section class="viewer" class:embed class:shot aria-label="{spec.title} in 3D">
	<div class="stage" class:ready={status === 'ready'} bind:this={stage}>
		<img class="poster" src={poster} alt="The Ark of the Covenant: a gold chest with two kneeling gold cherubim on its lid, staves through its rings" />
		{#if status === 'still'}
			<p class="still">This browser can't show the 3D model, so here is a still of it. Every part and its verses are listed beside it.</p>
		{/if}
		{#if status === 'ready'}
			<div class="spots">
				{#each spec.parts as p (p.id)}
					{@const at = spotFor(p.id)}
					{#if at && !building}
						<button
							type="button"
							class="spot"
							class:on={selected === p.id}
							style:transform="translate({at.x}px, {at.y}px)"
							aria-label="{partNumber(p.id)}. {p.label}"
							aria-pressed={selected === p.id}
							onclick={() => pick(p.id)}
						>
							<span>{partNumber(p.id)}</span>
						</button>
					{/if}
				{/each}
			</div>
			<p class="hint" aria-hidden="true">Drag to turn · pinch or scroll to zoom · one square is one cubit</p>
		{/if}
		{#if caption}
			<div class="caption" aria-live="polite">
				<p>
					<b>{refLabel(caption.ref)} · step {(step ?? 0) + 1} of {assembly.length}</b>
					{verse(caption.ref)}
				</p>
				<button type="button" class="skip" onclick={() => scene?.finish()}>Skip</button>
			</div>
		{/if}
	</div>

	<div class="controls">
		<Button kind="primary" icon={building ? undefined : 'cube'} label={building ? 'Show it built' : 'Build from the text'} onclick={build} disabled={status !== 'ready'} />
		<Segmented
			layout="fit"
			label="Cubit"
			options={cubits.cubits.map((c) => ({ value: c.id, label: c.label }))}
			bind:value={cubitId}
		/>
		<div class="switches">
			<label><Toggle label="Person for scale" bind:checked={person} /> Person for scale</label>
			<label><Toggle label="See inside" bind:checked={inside} /> See inside</label>
			<label><Toggle label="Close-up" bind:checked={closeUp} /> Close-up</label>
		</div>
	</div>

	<aside class="panel" aria-label="Verses">
		<div class="tr">
			<span>Verses in</span>
			{#if translations.length > 1}
				<SelectField
					label="Translation"
					options={translations.map((t) => ({ value: t.id, label: t.abbrev }))}
					value={texts.id}
					onchange={(id) => void useTranslation(id)}
				/>
			{:else}
				<b>{texts.abbrev}</b>
			{/if}
		</div>
		{#if missing}
			<p class="missing">{missing.toUpperCase()} isn't available here, so these verses are in the KJV.</p>
		{/if}

		{#if part}
			<header class="part">
				<h3><span class="n" aria-hidden="true">{partNumber(part.id)}</span>{part.label}</h3>
				<Badge kind={part.interpretive ? 'outline' : 'success'} label={part.interpretive ? 'Interpretive choice' : 'Stated in the text'} />
			</header>
			{#each partRefs(part) as ref (ref)}
				<blockquote>
					<p>{verse(ref)}</p>
					<cite><a href={linkFor(ref)} onclick={(e) => read(e, ref)}>{refLabel(ref)} ({abbrevOf(ref)})</a></cite>
				</blockquote>
			{/each}
			<p class="note">{part.note}</p>
			{#if part.options}
				{#each part.options as o (o.id)}
					<div class="option">
						<span>{o.label}</span>
						<Segmented
							layout="fit"
							label={o.label}
							options={o.choices.map((c) => ({ value: c.id, label: c.label }))}
							bind:value={choices[`${part.id}.${o.id}`]}
						/>
						<p class="note">{o.choices.find((c) => c.id === choices[`${part.id}.${o.id}`])?.note}</p>
					</div>
				{/each}
			{/if}
			<button type="button" class="back" onclick={() => pick(null)}><Icon name="chevron-left" size={16} /> All dimensions</button>
		{:else}
			<dl class="dims">
				{#each [['Length', chest.dims.length], ['Breadth', chest.dims.breadth], ['Height', chest.dims.height]] as const as [name, d] (name)}
					<div>
						<dt>{name}</dt>
						<dd><b>{cubitsLabel(d.value)} cubits</b> {len(d.value).inches} · {len(d.value).cm}</dd>
					</div>
				{/each}
				<div>
					<dt>A person</dt>
					<dd><b>{(cubits.person.inches / cubit.inches).toFixed(2)} cubits</b> {cubits.person.label}</dd>
				</div>
			</dl>
			<p class="note">{cubit.label}, {cubit.inches} in ({cubit.cm} cm). The text gives only cubits, so inches and centimetres are estimates.</p>
			<p class="hint-text">Tap a part, a number on the model or a name below to read the verses behind it.</p>
		{/if}

		<ol class="parts">
			{#each spec.parts as p (p.id)}
				<li>
					<button type="button" class:on={selected === p.id} aria-pressed={selected === p.id} onclick={() => pick(p.id)}>
						<span class="n" aria-hidden="true">{partNumber(p.id)}</span>
						{p.label}
						{#if p.interpretive}<span class="interp" title="Interpretive choice">Interpretive</span>{/if}
					</button>
				</li>
			{/each}
		</ol>
	</aside>
</section>

<style>
	.viewer {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 340px;
		grid-template-areas: 'stage panel' 'controls panel';
		grid-template-rows: auto 1fr;
		gap: 12px 20px;
		align-items: start;
	}

	.stage {
		grid-area: stage;
		position: relative;
		aspect-ratio: 16 / 10;
		max-height: 72vh;
		width: 100%;
		overflow: hidden;
		border-radius: 12px;
		background: #14110d;
		color: #f3ead8;
		isolation: isolate;
	}

	.stage :global(canvas.gl) {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		display: block;
		opacity: 0;
		transition: opacity 400ms;
		outline: none;
	}

	.stage.ready :global(canvas.gl) {
		opacity: 1;
	}

	.poster {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		transition: opacity 400ms;
	}

	.ready .poster {
		opacity: 0;
		pointer-events: none;
	}

	.still {
		position: absolute;
		left: 12px;
		right: 12px;
		bottom: 12px;
		padding: 8px 12px;
		border-radius: 10px;
		background: rgb(20 17 13 / 0.82);
		font: var(--type-small);
	}

	.hint {
		position: absolute;
		left: 14px;
		top: 12px;
		color: #b3a68c;
		font: 12px/16px var(--font-ui);
		pointer-events: none;
	}

	.spots {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}

	.spot {
		position: absolute;
		left: -13px;
		top: -13px;
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border-radius: 50%;
		pointer-events: auto;
		/* A larger target than the dot, for fingers. */
		outline-offset: 2px;
	}

	.spot::before {
		content: '';
		position: absolute;
		inset: -8px;
	}

	.spot span {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: rgb(20 17 13 / 0.78);
		border: 1.5px solid #e6b84f;
		color: #f3ead8;
		font: 600 11px/1 var(--font-ui);
		transition: transform var(--motion-fast);
	}

	.spot:hover span,
	.spot.on span {
		background: #e6b84f;
		color: #1a1408;
		transform: scale(1.12);
	}

	.caption {
		position: absolute;
		left: 50%;
		bottom: 16px;
		transform: translateX(-50%);
		display: flex;
		align-items: flex-start;
		gap: 12px;
		width: min(680px, calc(100% - 24px));
		padding: 10px 14px;
		border-radius: 12px;
		border: 1px solid #3a3024;
		background: rgb(20 17 13 / 0.86);
		font: 15px/1.5 var(--font-reading);
	}

	.caption b {
		display: block;
		margin-bottom: 2px;
		color: #e6b84f;
		font: 600 12px/16px var(--font-ui);
	}

	.skip {
		flex-shrink: 0;
		padding: 2px 10px;
		border-radius: 10px;
		border: 1px solid #3a3024;
		color: #b3a68c;
		font: 500 12px/18px var(--font-ui);
	}

	.skip:hover {
		color: #f3ead8;
	}

	.controls {
		grid-area: controls;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px 16px;
	}

	.switches {
		display: flex;
		flex-wrap: wrap;
		gap: 10px 16px;
	}

	.switches label {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		font: var(--type-ui);
		cursor: pointer;
	}

	.panel {
		grid-area: panel;
		align-self: stretch;
		min-width: 0;
		font: var(--type-ui);
	}

	.tr {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 12px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.missing {
		margin-bottom: 12px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.part {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 8px;
	}

	.part h3 {
		display: flex;
		align-items: center;
		gap: 8px;
		font: var(--type-ui-title);
		letter-spacing: var(--type-ui-title-tracking);
	}

	/* Over the reader's reset (read.css), which is as specific as a bare element here. */
	.panel blockquote {
		margin: 10px 0;
		padding: 2px 0 2px 12px;
		border-left: 2px solid var(--accent);
		font: var(--type-snippet);
		font-size: 15px;
		line-height: 24px;
	}

	cite {
		display: block;
		margin-top: 4px;
		font: var(--type-small);
		font-style: normal;
	}

	cite a {
		color: var(--accent);
	}

	.note {
		margin: 8px 0;
		color: var(--ink-2);
		font: var(--type-small);
		line-height: 18px;
	}

	.hint-text {
		color: var(--ink-2);
		font: var(--type-small);
	}

	.option {
		margin-top: 12px;
	}

	.option > span {
		display: block;
		margin-bottom: 6px;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.back {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		margin-top: 8px;
		color: var(--accent);
		font: var(--type-ui-strong);
	}

	.dims {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.dims > div {
		padding: 8px 10px;
		border-radius: 10px;
		background: var(--fill);
	}

	.dims dt {
		color: var(--ink-2);
		font: var(--type-label);
		letter-spacing: var(--type-label-tracking);
		text-transform: uppercase;
	}

	.dims dd {
		margin: 2px 0 0;
		color: var(--ink-2);
		font: var(--type-small);
	}

	.dims b {
		display: block;
		color: var(--ink);
		font: var(--type-ui-strong);
	}

	.parts {
		margin-top: 16px;
		padding-top: 12px;
		border-top: 1px solid var(--separator);
	}

	.parts button {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 6px 8px;
		margin: 0 -8px;
		border-radius: 8px;
		text-align: left;
	}

	.parts button:hover {
		background: var(--fill);
	}

	.parts button.on {
		background: var(--accent-soft);
	}

	.n {
		display: inline-grid;
		place-items: center;
		flex-shrink: 0;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		border: 1.5px solid var(--accent);
		color: var(--accent);
		font: 600 11px/1 var(--font-ui);
	}

	.interp {
		margin-left: auto;
		color: var(--ink-2);
		font: var(--type-small);
	}

	/* The Mac app's webview: the whole window, the panel scrolling on its own. */
	.embed {
		height: 100vh;
		grid-template-rows: minmax(0, 1fr) auto;
		padding: 16px;
		box-sizing: border-box;
	}

	.embed .stage {
		aspect-ratio: auto;
		max-height: none;
		height: 100%;
	}

	.embed .panel {
		overflow: auto;
		max-height: calc(100vh - 32px);
	}

	.shot .stage {
		position: fixed;
		inset: 0;
		z-index: 10;
		aspect-ratio: auto;
		max-height: none;
		border-radius: 0;
	}

	.shot :is(.spots, .hint, .poster) {
		display: none;
	}

	@media (max-width: 860px) {
		.viewer {
			grid-template-columns: minmax(0, 1fr);
			grid-template-areas: 'stage' 'controls' 'panel';
			grid-template-rows: auto;
		}

		.stage {
			aspect-ratio: 5 / 6;
			max-height: 64vh;
			border-radius: 0;
		}

		/* A narrow window (the Mac's sidebar open): one column the page scrolls, as on a phone. */
		.embed {
			height: auto;
			min-height: 100vh;
		}

		.embed .stage {
			height: max(60vh, 220px);
			border-radius: 12px;
		}

		.embed .panel {
			overflow: visible;
			max-height: none;
		}

		.caption {
			bottom: 10px;
			font-size: 14px;
			line-height: 1.45;
		}

		.hint {
			display: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.stage :global(canvas.gl),
		.poster,
		.spot span {
			transition: none;
		}
	}
</style>
