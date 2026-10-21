/**
 * The share card (share_sheet.rs `draw_card`): its size per shape, the verse set as large as fits
 * (21 pt down to 12, then cut short with "…" where the Mac lets it run off the card), and the same
 * layout drawn to a canvas for the image. Lines are broken here, with the canvas's measurements,
 * so the preview and the image wrap alike.
 */
import type { CardShape, CardStyle } from './settings';
import { ICONS, type IconDef, type IconName } from './ui/icons';

export interface Seg {
	text: string;
	red: boolean;
}

export interface CardColours {
	ground: string;
	ink: string;
	label: string;
	footer: string;
	brand: string;
	rubric: string;
}

export interface CardLayout {
	width: number;
	height: number;
	pad: number;
	size: number;
	lineHeight: number;
	lines: Seg[][];
	/** Whether the verses didn't all fit, even at 12 pt, and end in "…". */
	cut: boolean;
}

const SIZES: Record<CardShape, [number, number]> = {
	square: [300, 300],
	portrait: [300, 376],
	wide: [328, 205]
};

/** Literata's and Inter's line metrics (read.css's overrides), in ems. */
const SERIF = { ascent: 1.177, descent: 0.308 };
const SANS = { ascent: 0.96875, descent: 0.2412 };

export const CARD_FONT = '"Literata Heading"';
/** The Mac sets the verse this much lower than a CSS line box would. */
export const VERSE_DROP = 1;
const serif = (size: number) => `400 ${size}px ${CARD_FONT}`;

/** The card's fonts, loaded before any line is measured. */
export const cardFonts = () =>
	Promise.all([
		document.fonts.load(serif(21)),
		document.fonts.load('500 11px "Inter Reader"'),
		document.fonts.load('600 11px "Inter Reader"')
	]);

/** Where the baseline sits in a CSS line box `lh` tall. */
const baseline = (lh: number, size: number, m: typeof SERIF) => (lh - (m.ascent + m.descent) * size) / 2 + m.ascent * size;

let measurer: CanvasRenderingContext2D | null = null;
function ctx2d(): CanvasRenderingContext2D {
	measurer ??= document.createElement('canvas').getContext('2d')!;
	return measurer;
}

/** Words (runs of segments between spaces), keeping the words of Christ apart. */
function words(runs: Seg[]): Seg[][] {
	const out: Seg[][] = [];
	let word: Seg[] = [];
	for (const r of runs) {
		for (const part of r.text.split(/(\s+)/)) {
			if (!part) continue;
			if (/^\s+$/.test(part)) {
				if (word.length) out.push(word);
				word = [];
			} else {
				word.push({ text: part, red: r.red });
			}
		}
	}
	if (word.length) out.push(word);
	return out;
}

function wrap(ws: Seg[][], font: string, width: number): Seg[][] {
	const c = ctx2d();
	c.font = font;
	const space = c.measureText(' ').width;
	const measure = (w: Seg[]) => w.reduce((s, seg) => s + c.measureText(seg.text).width, 0);
	const lines: Seg[][][] = [];
	let line: Seg[][] = [];
	let x = 0;
	for (const w of ws) {
		const ww = measure(w);
		if (line.length && x + space + ww > width) {
			lines.push(line);
			line = [];
			x = 0;
		}
		x += (line.length ? space : 0) + ww;
		line.push(w);
	}
	if (line.length) lines.push(line);
	// Each line as segments, the spaces between words joined to the word before.
	return lines.map((l) => {
		const segs: Seg[] = [];
		l.forEach((w, i) => {
			w.forEach((seg, j) => {
				const text = seg.text + (i < l.length - 1 && j === w.length - 1 ? ' ' : '');
				const last = segs.at(-1);
				if (last && last.red === seg.red) last.text += text;
				else segs.push({ text, red: seg.red });
			});
		});
		return segs;
	});
}

export function layoutCard(shape: CardShape, runs: Seg[], reference: boolean): CardLayout {
	const [width, height] = SIZES[shape];
	const pad = height < 260 ? 24 : 32;
	const inner = width - 2 * pad;
	const top = reference ? 32 : 0;
	const room = height - 2 * pad - 16 - 16 - top;
	const ws = words(runs);
	let size = 21;
	let lines = wrap(ws, serif(size), inner);
	while (size > 12 && lines.length * Math.round((size * 32) / 21) > room) {
		size--;
		lines = wrap(ws, serif(size), inner);
	}
	const lineHeight = Math.round((size * 32) / 21);
	const keep = Math.max(1, Math.floor(room / lineHeight));
	const cut = lines.length > keep;
	if (cut) {
		// The most words that fit with "…" after them.
		const upTo = (n: number) => {
			const last = ws[n - 1];
			const end = last[last.length - 1];
			const ended = [...last.slice(0, -1), { ...end, text: `${end.text.replace(/[,;:.]+$/, '')}…` }];
			return wrap([...ws.slice(0, n - 1), ended], serif(size), inner);
		};
		let [lo, hi] = [1, ws.length];
		while (lo < hi) {
			const mid = Math.ceil((lo + hi) / 2);
			if (upTo(mid).length <= keep) lo = mid;
			else hi = mid - 1;
		}
		lines = upTo(lo);
	}
	return { width, height, pad, size, lineHeight, lines, cut };
}

/** A card style's colours, from the theme it borrows (`el` carries that theme's tokens). */
export function cardColours(style: CardStyle, el: Element): CardColours {
	const css = getComputedStyle(el);
	const v = (name: string) => css.getPropertyValue(name).trim();
	if (style === 'ink') {
		return {
			ground: v('--accent'),
			ink: '#ffffff',
			label: '#ffffff',
			footer: 'rgb(255 255 255 / 0.784)',
			brand: 'rgb(255 255 255 / 0.627)',
			rubric: 'rgb(255 196 186)'
		};
	}
	return { ground: v('--paper'), ink: v('--ink'), label: v('--accent'), footer: v('--ink-2'), brand: v('--ink-3'), rubric: v('--rubric') };
}

function icon(name: IconName, colour: string): Promise<HTMLImageElement> {
	const def: IconDef = ICONS[name];
	const units = def.units ?? 20;
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${units} ${units}" fill="none" stroke="${colour}" stroke-width="${def.stroke ?? 1.5}" stroke-linecap="round" stroke-linejoin="round">${def.body}</svg>`;
	const img = new Image();
	img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
	return img.decode().then(() => img);
}

/** The card as a PNG, `scale` pixels to the point, transparent outside its rounded corners. */
export async function cardImage(
	layout: CardLayout,
	colours: CardColours,
	text: { reference: string | null; translation: string | null },
	scale = 3
): Promise<Blob> {
	const { width, height, pad, size, lineHeight, lines } = layout;
	const canvas = document.createElement('canvas');
	canvas.width = width * scale;
	canvas.height = height * scale;
	const c = canvas.getContext('2d')!;
	c.scale(scale, scale);
	c.beginPath();
	c.roundRect(0, 0, width, height, 16);
	c.clip();
	c.fillStyle = colours.ground;
	c.fillRect(0, 0, width, height);
	c.textBaseline = 'alphabetic';

	let top = pad;
	if (text.reference) {
		c.font = '600 11px "Inter Reader"';
		c.letterSpacing = '1.32px';
		c.fillStyle = colours.label;
		c.fillText(text.reference.toUpperCase(), pad, top + baseline(16, 11, SANS));
		c.letterSpacing = '0px';
		top += 32;
	}
	c.font = serif(size);
	lines.forEach((line, i) => {
		let x = pad;
		const y = top + VERSE_DROP + i * lineHeight + baseline(lineHeight, size, SERIF);
		for (const seg of line) {
			c.fillStyle = seg.red ? colours.rubric : colours.ink;
			c.fillText(seg.text, x, y);
			x += c.measureText(seg.text).width;
		}
	});

	const footer = height - pad - 16;
	const y = footer + baseline(16, 11, SANS);
	if (text.translation) {
		c.font = '500 11px "Inter Reader"';
		c.fillStyle = colours.footer;
		c.fillText(text.translation, pad, y);
	}
	c.font = '600 11px "Inter Reader"';
	c.fillStyle = colours.brand;
	c.textAlign = 'right';
	c.fillText('Bible', width - pad, y);
	const brand = c.measureText('Bible').width;
	c.drawImage(await icon('book', colours.brand), width - pad - brand - 4 - 12, footer + 2, 12, 12);

	return new Promise((resolve, reject) =>
		canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('The image could not be made'))), 'image/png')
	);
}

/**
 * The copied text in two lines of `width` px (12 px monospace): a long quotation is shortened in
 * the middle so the reference after it stays in view.
 */
export function previewText(copy: string, width: number): string {
	const c = ctx2d();
	c.font = '12px ui-monospace, SFMono-Regular, Menlo, monospace';
	const flat = copy.replace(/\n/g, ' ');
	const rows = (s: string) => wrap(words([{ text: s, red: false }]), c.font, width).length;
	if (rows(flat) <= 2) return flat;
	const at = flat.lastIndexOf('” — ');
	if (at < 0) return flat;
	const quote = flat.slice(0, at);
	const tail = flat.slice(at + 4);
	const ws = quote.split(' ');
	for (let n = ws.length - 1; n >= 1; n--) {
		const s = `${ws.slice(0, n).join(' ').replace(/[,;:]+$/, '')}…” — ${tail}`;
		if (rows(s) <= 2) return s;
	}
	return flat;
}
