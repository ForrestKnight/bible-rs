import { SITE_NAME } from '#lib/config.js';
import { core } from '#lib/read/core.js';
import { ark, textsIn } from '#lib/objects/server.js';
import type { Cubit } from '#lib/objects/spec.js';
import type { PageServerLoad } from './$types';

/**
 * The Ark of the Covenant (/ark-of-the-covenant), built from objects/ark.json: the answer first,
 * the 3D viewer, the dimensions, every part with its KJV verses, and the questions people ask.
 * The text is all in the prerendered HTML; only the viewer needs the page's JavaScript.
 */
export const prerender = true;

/** 45, 27.5: inches to a tenth, without a trailing ".0". */
const inches = (n: number) => String(Number(n.toFixed(1)));

export const load: PageServerLoad = async () => {
	const { spec, cubits } = await ark();
	const kjv = await textsIn('kjv');
	const info = (await core()).info('kjv');
	const chest = spec.parts.find((p) => p.id === 'chest')!;
	const [L, B, H] = (['length', 'breadth', 'height'] as const).map((k) => chest.dims[k].value);
	const common = cubits.cubits.find((c) => c.id === 'common')!;
	const royal = cubits.cubits.find((c) => c.id === 'royal')!;
	const size = (c: Cubit) =>
		`${[L, B, H].map((n) => inches(n * c.inches)).join(' × ')} inches (${[L, B, H].map((n) => Math.round(n * c.cm)).join(' × ')} cm)`;
	const answer = `The Ark of the Covenant was 2½ cubits long, 1½ cubits wide and 1½ cubits high (Exodus 25:10). With the common cubit of ${common.inches} inches that is about ${size(common)}; with the royal cubit of ${royal.inches} inches, about ${size(royal)}.`;

	const faq: { question: string; answer: string; refs: string[] }[] = [
		{
			question: 'How big was the Ark of the Covenant?',
			answer: `${answer} The text gives the measurements in cubits only, so any figure in inches or centimetres is an estimate.`,
			refs: ['Exod.25.10', 'Exod.37.1']
		},
		{
			question: 'What was the Ark of the Covenant made of?',
			answer:
				'A chest of shittim (acacia) wood overlaid with pure gold inside and out, with a crown of gold around it. Its lid, the mercy seat, was pure gold, and the two cherubim on it were beaten gold, of one piece with the lid.',
			refs: ['Exod.25.10', 'Exod.25.11', 'Exod.25.17', 'Exod.25.18']
		},
		{
			question: 'What was inside the Ark of the Covenant?',
			answer:
				'The testimony: the two tables of stone (Exodus 25:16 and 31:18). Hebrews 9:4 adds the golden pot of manna and Aaron’s rod that budded; by the time of Solomon’s temple, 1 Kings 8:9 says only the two tables were in it.',
			refs: ['Exod.25.16', 'Exod.31.18', 'Heb.9.4', '1Kgs.8.9']
		},
		{
			question: 'How was the Ark carried?',
			answer:
				'On two staves of acacia wood overlaid with gold, put through four gold rings, two on each side, and never taken out. The text doesn’t give the staves’ length or say whether they ran along the length or the breadth; the model shows them along the length.',
			refs: ['Exod.25.12', 'Exod.25.13', 'Exod.25.14', 'Exod.25.15']
		},
		{
			question: 'What did the cherubim on the Ark look like?',
			answer:
				'The text says they were gold, of beaten work and of one piece with the mercy seat, one at each end, with their wings stretched out on high covering the mercy seat and their faces toward one another and toward the seat. It doesn’t describe their bodies or faces. The kneeling figures in the model follow artistic tradition and are labelled an interpretive choice.',
			refs: ['Exod.25.18', 'Exod.25.19', 'Exod.25.20', 'Exod.37.9']
		},
		{
			question: 'How long is a cubit?',
			answer: `A cubit is the length of a forearm, from the elbow to the fingertip. The common cubit is usually taken as about ${common.inches} inches (${Number(common.cm.toFixed(1))} cm). Ezekiel measures with a longer cubit, "a cubit and an hand breadth", often called the royal cubit, about ${royal.inches} inches (${Number(royal.cm.toFixed(1))} cm).`,
			refs: ['Ezek.40.5']
		},
		{
			question: 'Who made the Ark of the Covenant?',
			answer:
				'Bezaleel (Bezalel), the craftsman God named for the work of the tabernacle. Exodus 37 records him making the ark to the measurements given to Moses in Exodus 25.',
			refs: ['Exod.31.2', 'Exod.37.1']
		}
	];

	const path = `/${spec.slug}`;
	return {
		meta: {
			title: `Ark of the Covenant: Dimensions, 3D Model and Every Verse | ${SITE_NAME}`,
			description: `The Ark of the Covenant was 2½ × 1½ × 1½ cubits, about ${[L, B, H].map((n) => inches(n * common.inches)).join(' × ')} inches. See it built in 3D from Exodus 25, verse by verse, and read every verse behind it.`,
			path,
			crumbs: [
				{ name: 'Home', path: '/' },
				{ name: 'Ark of the Covenant', path }
			],
			image: `${path}/og.jpg`,
			faq: faq.map(({ question, answer }) => ({ question, answer }))
		},
		answer,
		faq,
		spec,
		cubits,
		kjv,
		notice: info?.copyright ?? ''
	};
};
