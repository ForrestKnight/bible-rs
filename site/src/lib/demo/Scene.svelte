<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import ComparePopover from '#lib/app/ComparePopover.svelte';
	import Icon, { type IconName } from '#lib/app/Icon.svelte';
	import NotePopover from '#lib/app/NotePopover.svelte';
	import Palette from '#lib/app/Palette.svelte';
	import SearchPanel from '#lib/app/SearchPanel.svelte';
	import VerseMenu from '#lib/app/VerseMenu.svelte';
	import Verses from '#lib/Verses.svelte';
	import { easeInOut, easeOut, envelope, span } from '#lib/ease.js';
	import { JOHN_13, JOHN_3, ROMANS_8 } from '#lib/scripture.js';
	import { CAPTIONS, RESULTS, T, places } from './script.js';

	let { t }: { t: number } = $props();

	const { jump, search, study, compare } = T;

	type Typing = { text: string; keys: readonly number[] };
	const typed = (typing: Typing) => typing.text.slice(0, typing.keys.filter((k) => t >= k).length);
	const blink = (typing: Typing) => typing.keys.some((k) => t >= k && t < k + 450) || Math.floor(t / 530) % 2 === 0;
	const mix = (color: string, p: number, under = 'transparent') =>
		p > 0 ? `color-mix(in srgb, var(${color}) ${(p * 100).toFixed(1)}%, ${under})` : undefined;

	// The passage on screen: Romans 8, then John 3 after the jump, then John 13 from the search.
	const PAGES = [
		{ title: 'Romans 8', verses: ROMANS_8, top: 58 },
		{ title: 'John 3', verses: JOHN_3, top: 66 },
		{ title: 'John 13', verses: JOHN_13, top: 92 }
	];
	const LANDS = [0, jump.land, search.land];
	let page = $derived(t < jump.land ? 0 : t < search.land ? 1 : 2);
	let shown = $derived(
		PAGES.map((_, i) => {
			if (i !== page) return 0;
			const enter = i === 0 ? span(t, 0, 300) : easeOut(span(t, LANDS[i], LANDS[i] + 320));
			const next = LANDS[i + 1];
			return next === undefined ? enter : Math.min(enter, 1 - span(t, next - 150, next));
		})
	);
	let scroll = $derived(easeInOut(span(t, T.scroll[0], T.scroll[1])) * 64);

	// The search sidebar pushes the reader and the toolbar's left buttons over.
	let side = $derived(
		easeInOut(span(t, search.sidebar, search.sidebar + 380)) - easeInOut(span(t, search.close, search.close + 380))
	);
	let readerLeft = $derived(260 + 144 * side);

	// ⌘K palette.
	let paletteShown = $derived(envelope(t, jump.open, jump.open + 200, jump.enter + 80, jump.enter + 260));
	let query = $derived(typed(jump.typing));
	let rows = $derived(places(query));
	let commands = $derived.by(() => {
		if (!query) return [];
		const list: { icon: IconName; label: string; keys: string[] }[] = [
			{ icon: 'search', label: `Search for “${query}”`, keys: ['⌘', 'F'] }
		];
		const verse = /^jn 3:(\d+)$/.exec(query)?.[1];
		if (verse) list.push({ icon: 'compare', label: `Compare John 3:${verse} across translations`, keys: ['C'] });
		if (rows.length) list.push({ icon: 'parallel', label: 'Open John 3 in parallel', keys: ['⇧', '⌘', 'P'] });
		return list;
	});

	// Search.
	let focus = $derived(span(t, search.focus, search.focus + 200) - span(t, search.close, search.close + 200));
	let searchText = $derived(t < search.close ? typed(search.typing) : '');
	let reveal = $derived(
		RESULTS.map((_, i) => easeOut(span(t, search.results + i * search.resultStep, search.results + i * search.resultStep + 220)))
	);
	let found = $derived(span(t, search.results - 120, search.results + 80));
	let pill = $derived(envelope(t, search.land + 120, search.land + 320, search.close, search.close + 200));

	// Study and compare, all on John 13:34.
	let menu = $derived(envelope(t, study.menu, study.menu + 180, study.menuOut, study.menuOut + 150));
	let note = $derived(envelope(t, study.note, study.note + 220, study.done + 60, study.done + 260));
	let compared = $derived(envelope(t, compare.open, compare.open + 220, compare.close, compare.close + 250));
	let compareRows = $derived(
		[0, 1].map((i) => easeOut(span(t, compare.rows + i * compare.rowStep, compare.rows + i * compare.rowStep + 220)))
	);

	let tints = $derived.by(() => {
		if (page === 1) {
			const [a, b] = jump.flash;
			return { 16: mix('--accent-soft', envelope(t, a, a + 200, b - 500, b)) };
		}
		if (page !== 2) return {};
		if (t >= study.swatch) return { 34: mix('--hl-yellow', span(t, study.swatch, study.swatch + 260), 'var(--accent-soft)') };
		if (t >= study.select) return { 34: mix('--accent-soft', span(t, study.select, study.select + 150)) };
		return { 34: mix('--accent-soft', envelope(t, search.land + 150, search.land + 350, search.close - 500, search.close)) };
	});

	// Where John 13:34 sits in the reader, so the menus open beside it.
	let anchor = $state({ top: 150, bottom: 250 });
	const measure: Attachment<HTMLElement> = (node) => {
		const observer = new ResizeObserver(() => {
			const verse = node.querySelector<HTMLElement>('[data-verse="34"]');
			if (verse) anchor = { top: verse.offsetTop, bottom: verse.offsetTop + verse.offsetHeight };
		});
		observer.observe(node);
		return () => observer.disconnect();
	};
	let verseTop = $derived(PAGES[2].top + anchor.top);
	let verseBottom = $derived(PAGES[2].top + anchor.bottom);

	let caption = $derived(CAPTIONS.find((c) => t >= c.start - 200 && t < c.end + 900) ?? null);
	let captionShown = $derived(
		caption ? envelope(t, caption.start - 200, caption.start, caption.end + 600, caption.end + 900) : 0
	);
</script>

<div class="scene">
	<div class="wallpaper"></div>

	<div class="window app">
		<div class="reader">
			{#each PAGES as p, i (p.title)}
				<div
					class="page"
					style:left="{readerLeft}px"
					style:top="{p.top}px"
					style:opacity={shown[i]}
					style:visibility={shown[i] > 0 ? 'visible' : 'hidden'}
					style:transform="translateY({i === 0 ? -scroll : (1 - shown[i]) * 8}px)"
					{@attach i === 2 ? measure : undefined}
				>
					{#if i === 0}
						<div class="heading"><span>ROMANS</span><b>8</b></div>
					{/if}
					<Verses verses={p.verses} tint={i === page ? tints : {}} />
				</div>
			{/each}
		</div>

		{#if page === 2 && t >= study.note}
			<div class="margin" style:left="{readerLeft + 540}px" style:top="{verseTop + 4}px">
				<Icon name="note" size={13} />
			</div>
		{/if}

		{#if pill > 0}
			<div class="pill" style:left="{readerLeft + 260}px" style:opacity={pill}>
				Result 1 of 19 <Icon name="down" size={10} /><span>Esc to clear</span>
			</div>
		{/if}

		{#if side > 0}
			<div class="sidebar" style:transform="translateX({(side - 1) * 290}px)">
				<SearchPanel {reveal} {found} selected={t >= search.pick ? 0 : -1} top={40} />
			</div>
		{/if}

		<div class="toolbar">
			<div class="cluster">
				<span class="lights"><i></i><i></i><i></i></span>
				<span class="moved" style:transform="translateX({side * 210}px)">
					<span class="button"><Icon name="sidebar" size={14} /></span>
					<span class="button pair"
						><span><Icon name="back" size={13} /></span><i></i><span class="off"
							><Icon name="forward" size={13} /></span
						></span
					>
				</span>
			</div>
			<div class="title" style:left="{readerLeft + 260}px">
				<span>{PAGES[page].title} <Icon name="down" size={9} /></span><i></i><span
					>KJV <Icon name="down" size={9} /></span
				>
			</div>
			<div class="cluster">
				<span class="button pair"
					><span><Icon name="parallel" size={14} /></span><span class="aa">A<small>A</small></span></span
				>
				<span
					class="search"
					style:width="{132 + 48 * focus}px"
					style:box-shadow="0 0 0 {focus}px var(--accent), 0 0 0 {focus * 3.5}px color-mix(in srgb, var(--accent) 25%, transparent), var(--control-edge)"
				>
					<Icon name="search" size={11} />
					{#if searchText}
						<span class="typed">{searchText}</span>
						<span class="caret" style:opacity={blink(search.typing) ? 1 : 0}></span>
					{:else}
						<span class="placeholder">Search</span>
					{/if}
				</span>
			</div>
		</div>

		{#if menu > 0}
			<div
				class="popover"
				style:left="{readerLeft + 250}px"
				style:top="{verseBottom + 6}px"
				style:opacity={menu}
				style:transform="translateY({(1 - menu) * -6}px) scale({0.97 + menu * 0.03})"
			>
				<VerseMenu picked={t >= study.swatch ? 0 : -1} />
			</div>
		{/if}

		{#if note > 0}
			<div
				class="popover"
				style:left="{readerLeft + 410}px"
				style:top="{verseTop + 31}px"
				style:opacity={note}
				style:transform="translateY({(1 - note) * -6}px) scale({0.97 + note * 0.03})"
			>
				<NotePopover
					reference="John 13:34"
					excerpt={JOHN_13[3].runs.map((r) => r.text).join('')}
					text={typed(study.typing)}
					caret={blink(study.typing)}
					pressed={t >= study.done && t < study.done + 160}
				/>
			</div>
		{/if}

		{#if compared > 0}
			<div
				class="popover compare"
				style:left="{readerLeft + 70}px"
				style:top="{verseBottom + 8}px"
				style:opacity={compared}
				style:transform="translateY({(1 - compared) * -6}px) scale({0.97 + compared * 0.03})"
			>
				<ComparePopover reveal={compareRows} rows={2} footer />
			</div>
		{/if}

		{#if paletteShown > 0}
			<div class="scrim" style:opacity={paletteShown}></div>
			<div
				class="palette"
				style:opacity={paletteShown}
				style:transform="translateY({(1 - paletteShown) * -8}px) scale({0.98 + paletteShown * 0.02})"
			>
				<Palette
					{query}
					caret={blink(jump.typing)}
					places={rows}
					{commands}
					pressed={t >= jump.enter && t < jump.enter + 160}
				/>
			</div>
		{/if}
	</div>

	<div class="menubar">
		<svg class="apple" viewBox="0 0 17 21" aria-hidden="true">
			<path
				d="M11.2 4.3c.6-.8 1-1.8.9-2.8-.9 0-2 .6-2.6 1.4-.6.7-1.1 1.7-.9 2.7 1 .1 2-.5 2.6-1.3zM12.1 5.8c-1.4-.1-2.7.8-3.3.8-.7 0-1.7-.8-2.9-.8C4.4 5.9 3 6.8 2.2 8.1c-1.6 2.7-.4 6.8 1.1 9 .8 1.1 1.7 2.3 2.8 2.3 1.1 0 1.6-.7 2.9-.7 1.4 0 1.8.7 2.9.7 1.2 0 2-1.1 2.7-2.2.9-1.3 1.2-2.5 1.2-2.6 0 0-2.4-.9-2.4-3.6 0-2.2 1.8-3.3 1.9-3.4-1-1.5-2.6-1.7-3.2-1.7z"
			/>
		</svg>
		<b>Bible</b>
		<span>File</span><span>View</span><span>Go</span><span>Window</span><span>Help</span>
		<span class="spacer"></span>
		<span>Tue Oct 20&nbsp;&nbsp;9:41 AM</span>
	</div>

	{#if caption && captionShown > 0}
		<div class="caption" style:opacity={captionShown} style:transform="translateY({(1 - captionShown) * 6}px)">
			{#each caption.words as w, i (i)}
				<span style:opacity={0.32 + 0.68 * span(t, w.at - 60, w.at + 140)}>{w.text}</span>
			{/each}
		</div>
	{/if}
</div>

<style>
	.scene {
		position: relative;
		width: 1200px;
		height: 720px;
		overflow: hidden;
		font-family: var(--sans);
		user-select: none;
	}

	.wallpaper {
		position: absolute;
		inset: 0;
		background-color: #0b0c16;
		background-image:
			radial-gradient(circle farthest-corner at 50% 46%, rgba(78, 98, 210, 0.34), transparent 60%),
			radial-gradient(circle farthest-corner at 2% 108%, rgba(150, 44, 70, 0.3), transparent 58%),
			radial-gradient(circle farthest-corner at 50% 50%, rgba(104, 84, 214, 0.26), transparent 60%),
			linear-gradient(158deg, #151a33 0%, #0b0d18 48%, #140f1e 100%);
	}

	.window {
		position: absolute;
		left: 80px;
		top: 62px;
		width: 1040px;
		height: 624px;
		overflow: hidden;
		border-radius: 12px;
		background: var(--paper);
		box-shadow:
			0 0 0 0.5px rgba(255, 255, 255, 0.18),
			0 30px 80px -10px rgba(0, 0, 0, 0.7);
		color: var(--ink);
	}

	/* Text scrolls up under the toolbar and fades out there. */
	.reader {
		position: absolute;
		inset: 0;
		mask-image: linear-gradient(transparent 44px, #000 60px);
	}

	.page {
		position: absolute;
		width: 520px;
		font: 15.5px/25px var(--serif);
		font-variation-settings: 'opsz' 12;
	}

	.heading {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		padding-bottom: 14px;
	}

	.heading span {
		color: var(--accent);
		font: 600 9px/13px var(--sans);
		letter-spacing: 0.12em;
	}

	.heading b {
		font: 300 39px/46px var(--serif);
	}

	.margin {
		position: absolute;
		color: var(--accent);
	}

	.pill {
		position: absolute;
		top: 50px;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 24px;
		padding: 0 12px;
		border-radius: 12px;
		background: var(--control);
		box-shadow: var(--control-edge);
		color: var(--ink-2);
		font-size: 10.5px;
		translate: -50% 0;
	}

	.pill span {
		margin-left: 6px;
		color: var(--ink-3);
	}

	.sidebar {
		position: absolute;
		top: 8px;
		bottom: 8px;
		left: 8px;
		width: 272px;
	}

	.toolbar {
		position: absolute;
		inset: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 42px;
		padding: 0 13px 0 16px;
		color: var(--ink);
		font-size: 11px;
	}

	.cluster,
	.moved {
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.lights {
		display: flex;
		gap: 7px;
		padding-right: 12px;
	}

	.lights i {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		background: #ff5f57;
	}

	.lights i:nth-child(2) {
		background: #febc2e;
	}

	.lights i:nth-child(3) {
		background: #28c840;
	}

	.button {
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 27px;
		height: 26px;
		border-radius: 13px;
		background: var(--control);
		box-shadow: var(--control-edge);
	}

	.pair > span {
		display: grid;
		place-items: center;
		width: 26px;
	}

	.pair > i {
		width: 1px;
		height: 13px;
		background: var(--separator);
	}

	.off {
		color: var(--ink-3);
	}

	.pair > .aa {
		display: flex;
		align-items: baseline;
		font: 600 12px var(--sans);
		letter-spacing: -0.04em;
	}

	.aa small {
		font-size: 9px;
	}

	.title {
		position: absolute;
		display: flex;
		align-items: center;
		gap: 9px;
		height: 26px;
		padding: 0 10px;
		border-radius: 13px;
		background: var(--control);
		box-shadow: var(--control-edge);
		font-weight: 600;
		translate: -50% 0;
	}

	.title span {
		display: flex;
		align-items: center;
		gap: 4px;
		white-space: nowrap;
	}

	.title :global(svg) {
		color: var(--ink-2);
	}

	.title > i {
		width: 1px;
		height: 13px;
		background: var(--separator);
	}

	.search {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 10px;
		border-radius: 13px;
		background: var(--field);
		color: var(--ink-2);
	}

	.typed {
		color: var(--ink);
		white-space: pre;
	}

	.placeholder {
		color: var(--ink-3);
	}

	.caret {
		width: 1px;
		height: 13px;
		margin-left: -6px;
		background: var(--accent);
	}

	.popover {
		position: absolute;
		transform-origin: 50% 0;
	}

	.compare {
		width: 380px;
	}

	.scrim {
		position: absolute;
		inset: 0;
		background: var(--scrim);
	}

	.palette {
		position: absolute;
		left: 276px;
		top: 78px;
		width: 488px;
		transform-origin: 50% 0;
	}

	.menubar {
		position: absolute;
		inset: 0 0 auto;
		display: flex;
		align-items: center;
		gap: 18px;
		height: 28px;
		padding: 0 16px;
		background: rgba(10, 10, 14, 0.32);
		backdrop-filter: blur(20px) saturate(160%);
		color: #f4f4f5;
		font-size: 13px;
	}

	.apple {
		width: 13px;
		height: 16px;
		fill: #f4f4f5;
	}

	.spacer {
		flex: 1;
	}

	.caption {
		position: absolute;
		left: 50%;
		bottom: 30px;
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 0 0.28em;
		max-width: 760px;
		padding: 9px 18px;
		border-radius: 14px;
		background: rgba(8, 8, 10, 0.62);
		backdrop-filter: blur(16px);
		color: #fff;
		font-size: 21px;
		font-weight: 600;
		letter-spacing: -0.01em;
		line-height: 28px;
		translate: -50% 0;
		white-space: nowrap;
	}
</style>
