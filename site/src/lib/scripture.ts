/**
 * Public-domain Scripture for the demo and the feature cards, as the app shows it: the words of
 * Christ in red, the KJV's translator additions in italics. Taken from the bundled translations
 * in data/translations. In the markup, {braces} mark the words of Christ and _underscores_ italics.
 */

export type Run = { text: string; red: boolean; italic: boolean; smallCaps: boolean };
export type Verse = { n: number; runs: Run[]; paragraph: boolean };

function parse(source: string): Run[] {
	const runs: Run[] = [];
	let red = false;
	let italic = false;
	let text = '';
	const flush = () => {
		// The divine name is set in small capitals, as the app does.
		for (const part of text.split(/(LORD)/)) {
			if (part) runs.push({ text: part === 'LORD' ? 'Lord' : part, red, italic, smallCaps: part === 'LORD' });
		}
		text = '';
	};
	for (const ch of source) {
		if (ch === '{' || ch === '}' || ch === '_') {
			flush();
			if (ch === '_') italic = !italic;
			else red = ch === '{';
		} else {
			text += ch;
		}
	}
	flush();
	return runs;
}

const verse = (n: number, source: string, paragraph = false): Verse => ({ n, runs: parse(source), paragraph });

/** Romans 8:1–11, KJV. */
export const ROMANS_8: Verse[] = [
	verse(1, '_There is_ therefore now no condemnation to them which are in Christ Jesus, who walk not after the flesh, but after the Spirit.', true),
	verse(2, 'For the law of the Spirit of life in Christ Jesus hath made me free from the law of sin and death.'),
	verse(3, 'For what the law could not do, in that it was weak through the flesh, God sending his own Son in the likeness of sinful flesh, and for sin, condemned sin in the flesh:'),
	verse(4, 'That the righteousness of the law might be fulfilled in us, who walk not after the flesh, but after the Spirit.'),
	verse(5, 'For they that are after the flesh do mind the things of the flesh; but they that are after the Spirit the things of the Spirit.'),
	verse(6, 'For to be carnally minded _is_ death; but to be spiritually minded _is_ life and peace.'),
	verse(7, 'Because the carnal mind _is_ enmity against God: for it is not subject to the law of God, neither indeed can be.'),
	verse(8, 'So then they that are in the flesh cannot please God.'),
	verse(9, 'But ye are not in the flesh, but in the Spirit, if so be that the Spirit of God dwell in you. Now if any man have not the Spirit of Christ, he is none of his.'),
	verse(10, 'And if Christ _be_ in you, the body _is_ dead because of sin; but the Spirit _is_ life because of righteousness.'),
	verse(11, 'But if the Spirit of him that raised up Jesus from the dead dwell in you, he that raised up Christ from the dead shall also quicken your mortal bodies by his Spirit that dwelleth in you.'),
];

/** John 3:14–21, KJV. */
export const JOHN_3: Verse[] = [
	verse(14, '{And as Moses lifted up the serpent in the wilderness, even so must the Son of man be lifted up:}', true),
	verse(15, '{That whosoever believeth in him should not perish, but have eternal life.}'),
	verse(16, '{For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.}', true),
	verse(17, '{For God sent not his Son into the world to condemn the world; but that the world through him might be saved.}'),
	verse(18, '{He that believeth on him is not condemned: but he that believeth not is condemned already, because he hath not believed in the name of the only begotten Son of God.}', true),
	verse(19, '{And this is the condemnation, that light is come into the world, and men loved darkness rather than light, because their deeds were evil.}'),
	verse(20, '{For every one that doeth evil hateth the light, neither cometh to the light, lest his deeds should be reproved.}'),
	verse(21, '{But he that doeth truth cometh to the light, that his deeds may be made manifest, that they are wrought in God.}'),
];

/** John 13:31–38, KJV. */
export const JOHN_13: Verse[] = [
	verse(31, 'Therefore, when he was gone out, Jesus said, {Now is the Son of man glorified, and God is glorified in him.}', true),
	verse(32, '{If God be glorified in him, God shall also glorify him in himself, and shall straightway glorify him.}'),
	verse(33, '{Little children, yet a little while I am with you. Ye shall seek me: and as I said unto the Jews, Whither I go, ye cannot come; so now I say to you.}'),
	verse(34, '{A new commandment I give unto you, That ye love one another; as I have loved you, that ye also love one another.}'),
	verse(35, '{By this shall all _men_ know that ye are my disciples, if ye have love one to another.}'),
	verse(36, 'Simon Peter said unto him, Lord, whither goest thou? Jesus answered him, {Whither I go, thou canst not follow me now; but thou shalt follow me afterwards.}', true),
	verse(37, 'Peter said unto him, Lord, why cannot I follow thee now? I will lay down my life for thy sake.'),
	verse(38, 'Jesus answered him, {Wilt thou lay down thy life for my sake? Verily, verily, I say unto thee, The cock shall not crow, till thou hast denied me thrice.}'),
];

/** Romans 8:28–30, KJV. */
export const ROMANS_8_28: Verse[] = [
	verse(28, 'And we know that all things work together for good to them that love God, to them who are the called according to _his_ purpose.'),
	verse(29, 'For whom he did foreknow, he also did predestinate _to be_ conformed to the image of his Son, that he might be the firstborn among many brethren.'),
	verse(30, 'Moreover whom he did predestinate, them he also called: and whom he called, them he also justified: and whom he justified, them he also glorified.'),
];

/** John 1:1–3, KJV. */
export const JOHN_1: Verse[] = [
	verse(1, 'In the beginning was the Word, and the Word was with God, and the Word was God.', true),
	verse(2, 'The same was in the beginning with God.'),
	verse(3, 'All things were made by him; and without him was not any thing made that was made.'),
];

/** John 13:34 in every bundled translation, the current one (KJV) first. */
export const COMPARE = [
	{ abbrev: 'KJV', text: 'A new commandment I give unto you, That ye love one another; as I have loved you, that ye also love one another.' },
	{ abbrev: 'WEB', text: 'A new commandment I give to you, that you love one another. Just as I have loved you, you also love one another.' },
	{ abbrev: 'BSB', text: 'A new commandment I give you: Love one another. As I have loved you, so you also must love one another.' },
	{ abbrev: 'ASV', text: 'A new commandment I give unto you, that ye love one another; even as I have loved you, that ye also love one another.' },
	{ abbrev: 'YLT', text: '\'A new commandment I give to you, that ye love one another; according as I did love you, that ye also love one another;' },
	{ abbrev: 'DBY', text: 'A new commandment I give to you, that ye love one another; as I have loved you, that ye also love one another.' },
	{ abbrev: 'WBS', text: 'A new commandment I give to you, That ye love one another; as I have loved you, that ye also love one another.' }
];

/** Psalm 23:1–4, BSB: poetry lines, each with its indent level. */
export const PSALM_23 = [
	{ n: 1, lines: [{ text: parse('The LORD is my shepherd;'), indent: 0 }, { text: parse('I shall not want.'), indent: 1 }] },
	{ n: 2, lines: [{ text: parse('He makes me lie down in green pastures;'), indent: 0 }, { text: parse('He leads me beside quiet waters.'), indent: 1 }] },
	{
		n: 3,
		lines: [
			{ text: parse('He restores my soul;'), indent: 0 },
			{ text: parse('He guides me in the paths of righteousness'), indent: 1 },
			{ text: parse('for the sake of His name.'), indent: 1 }
		]
	},
	{
		n: 4,
		lines: [
			{ text: parse('Even though I walk through the valley of the shadow of death,'), indent: 0 },
			{ text: parse('I will fear no evil,'), indent: 1 },
			{ text: parse('for You are with me;'), indent: 0 },
			{ text: parse('Your rod and Your staff, they comfort me.'), indent: 1 }
		]
	}
];
export const PSALM_23_HEADING = parse('The LORD Is My Shepherd');
