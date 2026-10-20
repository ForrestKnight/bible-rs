import type { Sound } from './sound.js';

/** The scene is laid out at this size and scaled to fit. */
export const STAGE = { w: 1200, h: 720 };
export const WINDOW = { x: 80, y: 62, w: 1040, h: 624 };

export const DURATION = 21_000;
/** With reduced motion the scene holds here: a highlighted verse with its note written. */
export const SETTLED = 16_000;

function typing(text: string, start: number, step: number) {
	return { text, start, keys: [...text].map((_, i) => start + i * step) };
}

export const T = {
	scroll: [600, 2600],
	jump: {
		open: 3200,
		typing: typing('jn 3:16', 3600, 130),
		enter: 4900,
		land: 5100,
		flash: [5250, 6900]
	},
	search: {
		focus: 7800,
		typing: typing('love one another', 8100, 75),
		sidebar: 8300,
		results: 9300,
		resultStep: 70,
		pick: 9900,
		land: 10_050,
		close: 11_700
	},
	study: {
		select: 12_200,
		menu: 12_500,
		swatch: 13_000,
		menuOut: 13_200,
		note: 13_700,
		typing: typing('Love as he loved us. See 1 John 4:11.', 14_000, 45),
		done: 16_100
	},
	compare: {
		open: 16_800,
		rows: 17_000,
		rowStep: 90,
		close: 20_300
	}
} as const;

export const CHAPTERS = [
	{ label: 'Read', t: 0 },
	{ label: 'Jump', t: 3000 },
	{ label: 'Search', t: 7600 },
	{ label: 'Study', t: 12_000 },
	{ label: 'Compare', t: 16_600 }
] as const;

export type Caption = { start: number; end: number; words: { text: string; at: number }[] };

/** Spreads the words across [start, end], giving longer words more time. */
function caption(text: string, start: number, end: number): Caption {
	const parts = text.split(' ');
	const weights = parts.map((w) => w.length + 2);
	const total = weights.reduce((a, b) => a + b, 0);
	let at = start;
	const words = parts.map((w, i) => {
		const word = { text: w, at };
		at += ((end - start) * weights[i]) / total;
		return word;
	});
	return { start, end, words };
}

export const CAPTIONS = [
	caption('It opens straight to the text.', 400, 2000),
	caption('Type a reference, and you’re already there.', 3500, 5500),
	caption('Every verse is searched as you type.', 8000, 9800),
	caption('Highlight a verse, and write a note beside it.', 12_300, 14_600),
	caption('Then compare it across translations.', 17_000, 18_800)
];

/** A palette row. `ref` is split into the parts the query matched (true) and the rest. */
export type Place = { ref: [string, boolean][]; detail: string; kind: 'verse' | 'chapter' };

const JOHN_3_CHAPTER: Place = { ref: [['J', true], ['oh', false], ['n 3', true]], detail: 'Chapter · 36 verses', kind: 'chapter' };

/** What the palette lists under "Go to" as the reference is typed. */
export function places(query: string): Place[] {
	if (query === 'jn 3:16')
		return [
			{
				ref: [['J', true], ['oh', false], ['n 3:16', true]],
				detail: 'For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.',
				kind: 'verse'
			},
			{
				ref: [['1 ', false], ['J', true], ['oh', false], ['n 3:16', true]],
				detail: 'Hereby perceive we the love of God, because he laid down his life for us: and we ought to lay down our lives for the brethren.',
				kind: 'verse'
			},
			JOHN_3_CHAPTER
		];
	if (query === 'jn 3:1')
		return [
			{
				ref: [['J', true], ['oh', false], ['n 3:1', true]],
				detail: 'There was a man of the Pharisees, named Nicodemus, a ruler of the Jews:',
				kind: 'verse'
			},
			JOHN_3_CHAPTER
		];
	if (query.startsWith('jn 3')) return [JOHN_3_CHAPTER];
	return [];
}

export const RESULTS = [
	{ ref: 'John 13:34', hits: '…give unto you, That ye [love one another]; as I have [loved] you, that ye also [love one another].' },
	{ ref: 'John 13:35', hits: '…my disciples, if ye have [love one] to [another].' },
	{ ref: 'John 15:12', hits: 'This is my commandment, That ye [love one another], as I have [loved] you.' },
	{ ref: 'John 15:17', hits: 'These things I command you, that ye [love one another].' },
	{ ref: 'Romans 12:10', hits: 'Be kindly affectioned [one] to [another] with brotherly [love]; in honour preferring [one another];' },
	{ ref: 'Romans 13:8', hits: 'Owe no man any thing, but to [love one another]: for he that [loveth another] hath fulfilled the law.' }
];

export type Cue = { t: number; sound: Sound; variant?: number };

const cue = (t: number, sound: Sound, variant?: number): Cue => ({ t, sound, variant });
const keys = (typed: { text: string; keys: readonly number[] }) =>
	typed.keys.map((t, i) => cue(t, typed.text[i] === ' ' ? 'space' : 'key', i));

export const CUES: Cue[] = [
	cue(T.jump.open, 'whoosh'),
	...keys(T.jump.typing),
	cue(T.jump.enter, 'enter'),
	cue(T.jump.land, 'sweep'),
	cue(T.search.focus, 'switch'),
	...keys(T.search.typing),
	cue(T.search.sidebar, 'whoosh'),
	...RESULTS.map((_, i) => cue(T.search.results + i * T.search.resultStep, 'tick')),
	cue(T.search.pick, 'enter'),
	cue(T.search.close, 'whoosh'),
	cue(T.study.select, 'tick'),
	cue(T.study.menu, 'pop'),
	cue(T.study.swatch, 'switch'),
	cue(T.study.note, 'pop'),
	...keys(T.study.typing),
	cue(T.study.done, 'enter'),
	cue(T.compare.open, 'pop'),
	...Array.from({ length: 5 }, (_, i) => cue(T.compare.rows + i * T.compare.rowStep, 'tick'))
].sort((a, b) => a.t - b.t);
