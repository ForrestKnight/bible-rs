import { json } from '@sveltejs/kit';
import { core } from '#lib/read/core.js';
import { contentOptions, defaultTypography } from '#lib/read/settings.js';
import type { RequestHandler } from './$types';

/**
 * The first launch's samples (onboarding.rs `samples`): John 3:16 in each offered translation, and
 * Psalm 23's heading and opening verses (BSB) for the theme previews. Built once, so the welcome
 * doesn't download six translations to show a verse from each.
 */
export const prerender = true;

const OFFERED = ['kjv', 'web', 'bsb', 'asv', 'ylt', 'dby'];
const text = (runs: { text: string }[]) => runs.map((r) => r.text).join('');

export const GET: RequestHandler = async () => {
	const c = await core();
	const offered = OFFERED.filter((id) => c.bundled.includes(id));
	const translations = await Promise.all(offered.map((id) => c.load(id)));
	const index = (book: number, chapter: number) => c.books[book].firstChapter + chapter - 1;
	const john = translations.map((t) => ({
		id: t.id,
		abbrev: t.abbrev,
		name: t.name,
		year: t.year,
		verse: text(c.verse(t.id, { book: 42, chapter: 3, verse: 16 })?.runs ?? []).trim()
	}));

	// The psalm's opening line keeps a line of its own; the rest runs on.
	const psalm = c.chapter('bsb', index(18, 23), contentOptions(defaultTypography()));
	const isHeading = (b: (typeof psalm.blocks)[number]) => b.kind.type === 'heading' || b.kind.type === 'psalmHeading';
	const heading = text(psalm.blocks.find(isHeading)?.runs ?? []).trim();
	const parts: string[] = [];
	let firstLine = true;
	for (const b of psalm.blocks) {
		const runs = b.runs.filter((r) => !r.number && r.verse >= 1 && r.verse <= 4);
		if (!runs.length || isHeading(b)) continue;
		const t = text(runs).replace(/\s+/g, ' ').trim();
		if (firstLine && runs.every((r) => r.verse === 1)) {
			parts.push(`${t}\n`);
			firstLine = false;
		} else parts.push(`${t} `);
	}
	return json({ john, psalm: { heading, text: parts.join('').trim() } });
};
