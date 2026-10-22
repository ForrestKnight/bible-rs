/**
 * The Mac app's line icons (crates/app/src/icons.rs) as SVG: a 20-unit grid with a 16-unit live
 * area and 1.5-unit round strokes (1.75 when heavy), scaled with the box as the design's SVGs are.
 * The disclosure chevrons use their own 12-unit grid; a few glyphs fix their own weight.
 * `.f` parts are filled; `.cut` is knocked out of a filled shape in the background colour.
 */
export interface IconDef {
	/** The grid: 20 units unless noted. */
	units?: 12;
	/** A weight that doesn't follow `heavy`. */
	stroke?: number;
	body: string;
}

const rect = (x: number, y: number, w: number, h: number, rx: number) =>
	`<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}"/>`;
const circle = (cx: number, cy: number, r: number, fill = false) =>
	`<circle cx="${cx}" cy="${cy}" r="${r}"${fill ? ' class="f"' : ''}/>`;
const path = (d: string, cls = '') => `<path d="${d}"${cls ? ` class="${cls}"` : ''}/>`;

const doc = path('M5.25 2.75H11.5L14.75 6V17.25H5.25Z') + path('M11.5 2.75V6H14.75');
const tray = path('M3.25 12V14.75A2 2 0 0 0 5.25 16.75H14.75A2 2 0 0 0 16.75 14.75V12');
const bookmark = 'M5.75 3.25H14.25V16.75L10 13.25 5.75 16.75Z';

const sunRays = Array.from({ length: 8 }, (_, k) => {
	const a = (k * Math.PI) / 4;
	const p = (r: number) => `${+(10 + r * Math.cos(a)).toFixed(3)} ${+(10 + r * Math.sin(a)).toFixed(3)}`;
	return `M${p(5.6)}L${p(7.4)}`;
}).join('');

export const ICONS = {
	sidebar: { body: rect(2.75, 4.25, 14.5, 11.5, 2.5) + path('M7.75 4.25V15.75') },
	columns: { body: rect(2.75, 4.25, 14.5, 11.5, 2.5) + path('M10 4.25V15.75') },
	'chevron-left': { body: path('M12 4.5 6.5 10 12 15.5') },
	'chevron-right': { body: path('M8 4.5 13.5 10 8 15.5') },
	'chevron-down': { units: 12, stroke: 1.5, body: path('M3 4.5 6 7.5 9 4.5') },
	'chevron-up': { units: 12, stroke: 1.5, body: path('M3 7.5 6 4.5 9 7.5') },
	disclosure: { units: 12, stroke: 1.5, body: path('M4.5 3 7.5 6 4.5 9') },
	'chevrons-up-down': { body: path('M6.5 8 10 4.5 13.5 8M6.5 12 10 15.5 13.5 12') },
	'text-size': {
		body: path('M2.75 15.5 7 4.5 11.25 15.5M4.4 11.5H9.6M12.5 15.5 15 9.25 17.5 15.5M13.4 13.4H16.6')
	},
	'cloud-slash': {
		body:
			path('M6 15.25H14.25A3.25 3.25 0 0 0 14.75 8.79A4.75 4.75 0 0 0 5.6 8.1A3.6 3.6 0 0 0 6 15.25Z') +
			path('M3.25 3.25 16.75 16.75')
	},
	expand: {
		body: path('M3.25 7.25V3.25H7.25M16.75 7.25V3.25H12.75M3.25 12.75V16.75H7.25M16.75 12.75V16.75H12.75')
	},
	'search-thin': { body: circle(8.75, 8.75, 5.25) + path('M12.75 12.75 16.75 16.75') },
	search: { stroke: 1.75, body: circle(8.75, 8.75, 5.25) + path('M12.75 12.75 16.75 16.75') },
	bookmark: { body: path(bookmark) },
	'bookmark-fill': { body: path(bookmark, 'fs') },
	highlighter: { body: path('M11.75 3.75 16.25 8.25 8.75 15.75H4.25V11.25Z') + path('M9.5 6 14 10.5') },
	note: { body: rect(4, 2.75, 12, 14.5, 2.25) + path('M7.25 7H12.75M7.25 10H12.75M7.25 13H10.25') },
	book: {
		body: path(
			'M4.25 15.5V4.75A2 2 0 0 1 6.25 2.75H15.75V15.25H6.25A2 2 0 0 0 4.25 17.25A2 2 0 0 0 6.25 19.25H15.75'
		)
	},
	close: { body: path('M5.5 5.5 14.5 14.5M14.5 5.5 5.5 14.5') },
	clock: { body: circle(10, 10, 7.25) + path('M10 6V10L12.75 11.75') },
	clear: {
		body: circle(10, 10, 10, true) + path('M7.1875 7.1875 12.8125 12.8125M12.8125 7.1875 7.1875 12.8125', 'cut')
	},
	list: {
		body:
			path('M7.75 5.5H16.75M7.75 10H16.75M7.75 14.5H16.75') +
			circle(4, 5.5, 0.75) +
			circle(4, 10, 0.75) +
			circle(4, 14.5, 0.75)
	},
	compare: { body: path('M3.25 7H15.75M12.75 4 15.75 7 12.75 10M16.75 13H4.25M7.25 10 4.25 13 7.25 16') },
	history: {
		body: path('M3.6 8.2A6.75 6.75 0 1 1 3.25 10') + path('M2.75 4.75V8.5H6.5M10 6.5V10.25L12.5 11.75')
	},
	copy: { body: rect(6.75, 6.75, 10.5, 10.5, 2.25) + path('M13.25 3.25H5.75A2.5 2.5 0 0 0 3.25 5.75V13.25') },
	plus: { body: path('M10 4.25V15.75M4.25 10H15.75') },
	quote: {
		body:
			path('M4.25 11.5Q4.25 7 8.25 5.5M11.25 11.5Q11.25 7 15.25 5.5') + circle(6, 13, 1.75) + circle(13, 13, 1.75)
	},
	share: {
		body:
			path('M10 2.75V12.75M6.75 6 10 2.75 13.25 6') +
			path(
				'M6.5 8.75H5.75A1.5 1.5 0 0 0 4.25 10.25V15.75A1.5 1.5 0 0 0 5.75 17.25H14.25A1.5 1.5 0 0 0 15.75 15.75V10.25A1.5 1.5 0 0 0 14.25 8.75H13.5'
			)
	},
	'text-cursor': { body: path('M10 4V16M7.5 4H12.5M7.5 16H12.5') },
	'slash-circle': { body: circle(10, 10, 7.25) + path('M4.9 15.1 15.1 4.9') },
	trash: {
		body: path(
			'M3.75 5.5H16.25M8 5.5V3.75H12V5.5M5.25 5.5 6 16.25 7 17.2H13L14 16.25 14.75 5.5'
		)
	},
	sun: { body: circle(10, 10, 3.25) + path(sunRays) },
	keyboard: {
		body:
			rect(2.25, 5.25, 15.5, 10, 2) +
			path('M5.5 8.5H6M8.25 8.5H8.75M11.25 8.5H11.75M14 8.5H14.5M5.5 11.75H6M14 11.75H14.5M8.25 11.75H11.75')
	},
	tray: {
		body:
			path('M2.75 11.25 5 4.75H15L17.25 11.25V15.25H2.75Z') +
			path('M2.75 11.25H6.75L7.75 13.25H12.25L13.25 11.25H17.25')
	},
	check: { stroke: 2.5, body: path('M4.75 10.5 8.25 14 15.25 6') },
	'arrow-right': { body: path('M3.75 10H16.25M11.75 5.5 16.25 10 11.75 14.5') },
	download: { body: path('M10 2.75V12.25M6.5 8.75 10 12.25 13.5 8.75') + tray },
	upload: { body: path('M10 12.25V2.75M6.5 6.25 10 2.75 13.5 6.25') + tray },
	globe: {
		body:
			circle(10, 10, 7.25) +
			path('M2.75 10H17.25') +
			path('M10 2.75C12 4.75 13 7.25 13 10C13 12.75 12 15.25 10 17.25C8 15.25 7 12.75 7 10C7 7.25 8 4.75 10 2.75Z')
	},
	checklist: {
		body: path(
			'M3.5 5.5 4.75 6.75 7 4.25M10 5.5H16.5M3.5 10.5 4.75 11.75 7 9.25M10 10.5H16.5M10 15.5H16.5M4 15.5H6.25'
		)
	},
	key: { body: circle(7, 12.5, 3.75) + path('M9.75 9.75 16.25 3.25M14.25 5.25 16.25 7.25M12.5 7 14 8.5') },
	link: {
		body:
			path('M8.506 11.505A3 3 0 0 0 12.749 11.5L15.25 9A3.005 3.005 0 0 0 15.25 4.75A3.005 3.005 0 0 0 11 4.75L10.25 5.5') +
			path('M11.494 8.495A3 3 0 0 0 7.251 8.5L4.75 11A3.005 3.005 0 0 0 4.75 15.25A3.005 3.005 0 0 0 9 15.25L9.75 14.5')
	},
	calendar: { body: rect(3, 4.25, 14, 12.5, 2.25) + path('M3 8.25H17M6.75 2.75V5.75M13.25 2.75V5.75') },
	flame: {
		body: path(
			'M10 17.25C7 17.25 4.75 15.15 4.75 12.25C4.75 8.65 8 6.85 8.5 3.25C11 4.85 12.5 6.95 12.75 9.25C13.5 8.75 14 7.85 14.25 7C15.5 8.3 16 10 16 12.25C16 15.15 13.5 17.25 10 17.25Z'
		)
	},
	moon: { body: path('M16.259 12.399A6.75 6.75 0 0 1 7.598 3.753A6.75 6.75 0 1 0 16.251 12.404Z') },
	auto: { body: circle(10, 10, 7.25) + path('M10 2.75V17.25') + path('M10 4.5A5.5 5.5 0 0 1 10 15.5Z', 'f') },
	doc: { body: doc },
	page: { body: doc + path('M7.75 10H12.25M7.75 13H12.25') },
	folder: {
		body: path(
			'M2.75 6.25A1.5 1.5 0 0 1 4.25 4.75H7.75L9.5 6.75H15.75A1.5 1.5 0 0 1 17.25 8.25V14.75A1.5 1.5 0 0 1 15.75 16.25H4.25A1.5 1.5 0 0 1 2.75 14.75Z'
		)
	},
	external: {
		body:
			path('M8.25 4.25H5.25A1.5 1.5 0 0 0 3.75 5.75V14.75A1.5 1.5 0 0 0 5.25 16.25H14.25A1.5 1.5 0 0 0 15.75 14.75V11.75') +
			path('M11.25 3.25H16.75V8.75M16.5 3.5 9.25 10.75')
	},
	contract: {
		body: path('M7.25 3.25V7.25H3.25M12.75 3.25V7.25H16.75M7.25 16.75V12.75H3.25M12.75 16.75V12.75H16.75')
	},
	lock: { body: rect(4.25, 8.75, 11.5, 8.5, 2) + path('M6.75 8.75V6.5A3.25 3.25 0 0 1 13.25 6.5V8.75') },
	warning: {
		body:
			path('M8.698 3.605A1.5 1.5 0 0 1 11.302 3.605L17.302 14.005A1.5 1.5 0 0 1 16 16.25H4A1.5 1.5 0 0 1 2.698 14.005Z') +
			path('M10 7.75V11') +
			circle(10, 13.65, 0.85, true)
	},
	retry: { body: path('M15.968 11.26A6.1 6.1 0 1 1 14.283 5.657') + path('M14.75 2.75V6.25H11.25') },
	info: { body: circle(10, 10, 7.25) + path('M10 9.25V13.75') + circle(10, 6.45, 0.9, true) },
	ellipsis: { body: circle(5, 10, 1.4, true) + circle(10, 10, 1.4, true) + circle(15, 10, 1.4, true) },
	/** An object modelled in 3D (the glyph after a verse that describes one). */
	cube: { body: path('M10 2.75 16.25 6.25V13.75L10 17.25 3.75 13.75V6.25Z') + path('M3.75 6.25 10 9.75 16.25 6.25M10 9.75V17.25') }
} satisfies Record<string, IconDef>;

export type IconName = keyof typeof ICONS;
