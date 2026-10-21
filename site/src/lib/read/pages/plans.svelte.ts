/** Reading plans' progress as of today (pages/plans.rs `Under`), and the page's state. */
import { now, type ReadApp } from '../app.svelte';
import { planCatalogue, planDays, type PlanDay, type PlanInfo, type PlanProgress } from '../core';
import { dayStart } from '../time';
import type { IconName } from '../ui';

export const DAY = 86_400;
/** Reading speed for time estimates, in words a minute. */
const WPM = 230;

export const MONTHS = [
	'January',
	'February',
	'March',
	'April',
	'May',
	'June',
	'July',
	'August',
	'September',
	'October',
	'November',
	'December'
];

/** Kept across visits to the page, as the Mac keeps it. */
export const plansPage = $state<{ tab: 'active' | 'browse' | 'completed'; open: { id: string; day: number } | null }>({
	tab: 'active',
	open: null
});

let catalogue: PlanInfo[] | null = null;
const days = new Map<string, PlanDay[]>();

export const plans = () => (catalogue ??= planCatalogue());
export const plan = (id: string) => plans().find((p) => p.id === id);

export function daysOf(id: string): PlanDay[] {
	let d = days.get(id);
	if (!d) days.set(id, (d = planDays(id)));
	return d;
}

export const ICONS: Record<PlanInfo['icon'], IconName> = {
	calendar: 'calendar',
	book: 'book',
	quote: 'quote',
	history: 'history',
	flame: 'flame',
	note: 'note'
};

export type Passage = PlanDay['passages'][number];

/** A plan with its progress. */
export class Under {
	readonly days: PlanDay[];
	readonly reads: Set<string>;
	/** Chapters read to the end since the plan began. */
	readonly finished: Set<number>;
	/** Today's day number (1-based; beyond the plan's length once it's over). */
	readonly today: number;

	constructor(
		readonly info: PlanInfo,
		readonly state: PlanProgress,
		app: ReadApp
	) {
		this.days = daysOf(info.id);
		this.reads = app.planReads(info.id);
		this.finished = new Set(
			app
				.readingSince(state.started)
				.filter((s) => s.verses > 0 && s.furthest >= s.verses)
				.map((s) => (app.core?.books[s.book]?.firstChapter ?? 0) + s.chapter - 1)
		);
		this.today = Math.max(0, Math.floor((dayStart(now()) - state.started) / DAY)) + 1;
	}

	get len() {
		return this.days.length;
	}

	day(d: number): Passage[] {
		return this.days[d - 1]?.passages ?? [];
	}

	passageDone(d: number, i: number): boolean {
		const p = this.day(d)[i];
		return this.reads.has(`${d}:${i}`) || (!!p && p.verses === null && this.finished.has(p.chapterIndex));
	}

	dayDone(d: number): boolean {
		const n = this.day(d).length;
		return n > 0 && this.day(d).every((_, i) => this.passageDone(d, i));
	}

	doneDays(): number {
		let n = 0;
		for (let d = 1; d <= this.len; d++) if (this.dayDone(d)) n++;
		return n;
	}

	/** Days read in a row, up to today (or yesterday, while today is still to do). */
	streak(): number {
		let d = Math.min(this.today, this.len);
		if (!this.dayDone(d)) d--;
		let n = 0;
		while (d >= 1 && this.dayDone(d)) {
			n++;
			d--;
		}
		return n;
	}

	/** The day to continue with: today's, or the first unfinished one before it. */
	currentDay(): number {
		const today = Math.max(1, Math.min(this.today, this.len));
		if (this.dayDone(today)) return today;
		for (let d = 1; d < today; d++) if (!this.dayDone(d)) return today - d <= 1 ? d : today;
		return today;
	}

	/** The first passage of a day still to read. */
	next(d: number): Passage | undefined {
		const ps = this.day(d);
		return ps[ps.findIndex((_, i) => !this.passageDone(d, i))] ?? ps[0];
	}
}

/** Verses in a passage and the words in them (as the reading translation has them). */
export function passageSize(app: ReadApp, p: Passage): [number, number] {
	const verses = app.verses(app.settings.translation, p.chapterIndex) ?? [];
	const [a, b] = p.verses ?? [1, 255];
	const shown = verses.filter((v) => v.number >= a && v.number <= b);
	const words = shown.reduce(
		(n, v) => n + v.runs.map((r) => r.text).join('').split(/\s+/).filter(Boolean).length,
		0
	);
	return [shown.length, words];
}

export const minutes = (words: number) => Math.max(1, Math.ceil(words / WPM));

export function openPassage(app: ReadApp, p: Passage) {
	const verse = p.verses?.[0];
	app.dispatch({ type: 'jump', chapter: p.chapterIndex, verse, flash: verse !== undefined });
}
