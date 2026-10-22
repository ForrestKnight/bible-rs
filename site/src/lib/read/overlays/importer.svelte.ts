/**
 * The import sheet's state (import_sheet.rs `ImportSheet`), kept outside the sheet so a conversion
 * goes on while the sheet is put away: choose a source, convert it in the worker, then review
 * anything that needs a look before it joins the library.
 */
import type { ReadApp } from '../app.svelte';
import {
	cancelConversion,
	convertSource,
	peekSource,
	pickSource,
	type ImportProgress,
	type ImportResult,
	type Peek,
	type Picked
} from '../imports';

export type Stage = 'choose' | 'converting' | 'review';

export type Issue =
	| { kind: 'locked' }
	| { kind: 'failed'; error: string }
	| { kind: 'missing'; books: number[] }
	| { kind: 'skipped'; books: string[] }
	| { kind: 'warnings'; list: string[] };

class Importer {
	stage = $state<Stage>('choose');
	picked = $state.raw<Picked | null>(null);
	peek = $state.raw<Peek | null>(null);
	peekError = $state<string | null>(null);
	abbrev = $state('');
	name = $state('');
	key = $state('');
	/** Files are being dragged over the page. */
	dragging = $state(false);
	progress = $state<ImportProgress>({ step: 0, books: 0, book: -1 });
	/** When each step began (ms), to show how long the finished ones took. */
	started = $state<number[]>([]);
	result = $state.raw<ImportResult | null>(null);
	error = $state<string | null>(null);
	keyTried = $state<string | null>(null);
	#run = 0;

	get displayName(): string {
		return this.name.trim() || this.abbrev;
	}

	get ready(): boolean {
		return !!this.peek && !!this.abbrev.trim();
	}

	/** What the review shows. */
	get issues(): Issue[] {
		return issues(this.result, this.error);
	}

	/** Takes a source and reads what it is. */
	async choose(picked: Picked) {
		this.picked = picked;
		this.peek = null;
		this.peekError = null;
		try {
			const p = await peekSource(picked);
			if (this.picked !== picked) return;
			this.peek = p;
			this.abbrev = p.abbrev;
			this.name = p.name;
		} catch (e) {
			if (this.picked === picked) this.peekError = message(e);
		}
	}

	async pick(folder = false) {
		const picked = await pickSource(folder);
		if (picked) await this.choose(picked);
	}

	/** Converts the chosen source; straight into the library when nothing needs a look. */
	async start(app: ReadApp) {
		const picked = this.picked;
		if (!picked) return;
		const run = ++this.#run;
		this.stage = 'converting';
		this.progress = { step: 0, books: 0, book: -1 };
		this.started = [performance.now()];
		const key = this.key.trim();
		try {
			const result = await convertSource(
				picked,
				{ abbrev: this.abbrev.trim(), name: this.name.trim(), unlockKey: key },
				app.takenIds(),
				(p) => {
					if (run !== this.#run) return;
					this.progress = { ...this.progress, ...p };
					while (this.started.length <= this.progress.step) this.started.push(performance.now());
				}
			);
			if (run !== this.#run) return;
			this.progress = { ...this.progress, step: 3 };
			if (issues(result, null).length === 0) {
				await this.install(app, result);
				return;
			}
			this.result = result;
			this.error = null;
		} catch (e) {
			if (run !== this.#run) return;
			this.result = null;
			this.error = message(e);
		}
		this.keyTried = key || null;
		this.stage = 'review';
		app.busy = null;
		app.importSheet = true;
	}

	async install(app: ReadApp, result: ImportResult) {
		this.reset();
		app.busy = null;
		app.importSheet = false;
		await app.installImported(result);
	}

	/** Puts the sheet away while a conversion goes on (Escape, or a click beside it). */
	hide(app: ReadApp) {
		app.importSheet = false;
		app.busy = `Importing ${this.displayName}…`;
	}

	cancel(app: ReadApp) {
		this.#run++;
		cancelConversion();
		this.close(app);
	}

	close(app: ReadApp) {
		this.reset();
		app.busy = null;
		app.importSheet = false;
	}

	/** Back to choosing, keeping the source (Choose Another File). */
	again() {
		this.stage = 'choose';
		this.result = null;
		this.error = null;
	}

	reset() {
		this.#run++;
		this.stage = 'choose';
		this.picked = null;
		this.peek = null;
		this.peekError = null;
		this.abbrev = this.name = this.key = '';
		this.dragging = false;
		this.result = null;
		this.error = null;
		this.keyTried = null;
	}

	/** Screenshots: converting, `books` of 66 done (import_sheet.rs `demo_converting`). */
	demoConverting(books: number) {
		this.#run++;
		const now = performance.now();
		this.stage = 'converting';
		this.progress = { step: 2, books, book: Math.min(books, 66) - 1 };
		this.started = [now - 1600, now - 1400, now - 1350];
	}

	/** Screenshots: the review after a failed import, or (no error) after a real one. */
	async demoReview(app: ReadApp, error: string | null, key: string | null) {
		if (key) this.key = key;
		if (error) {
			this.#run++;
			this.result = null;
			this.error = error;
			this.keyTried = key;
			this.stage = 'review';
			return;
		}
		const picked = this.picked;
		if (!picked) return;
		try {
			this.result = await convertSource(picked, {}, app.takenIds(), () => {});
			this.error = null;
		} catch (e) {
			this.result = null;
			this.error = message(e);
		}
		this.keyTried = key;
		this.stage = 'review';
	}
}

export const importer = new Importer();

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

export const isLockError = (e: string) => e.includes('locked') || e.includes('unlock key');

/** What a finished (or failed) import needs looked at. */
export function issues(result: ImportResult | null, error: string | null): Issue[] {
	if (error !== null) return [isLockError(error) ? { kind: 'locked' } : { kind: 'failed', error }];
	if (!result) return [];
	const out: Issue[] = [];
	if (result.missing.length) out.push({ kind: 'missing', books: result.missing });
	if (result.skippedBooks.length) out.push({ kind: 'skipped', books: result.skippedBooks });
	if (result.warnings.length) out.push({ kind: 'warnings', list: result.warnings });
	return out;
}

/** "SWORD module · zText · 4.1 MB · KJV versification". */
export function summary(p: Peek): string {
	const v = (d: string) => d.endsWith('versification');
	return [p.format, ...p.details.filter((d) => !v(d)), `${(p.bytes / 1_048_576).toFixed(1)} MB`, ...p.details.filter(v)].join(
		' · '
	);
}

const DEUTERO: Record<string, string> = {
	TOB: 'Tobit',
	JDT: 'Judith',
	JDTH: 'Judith',
	WIS: 'Wisdom',
	SIR: 'Sirach',
	BAR: 'Baruch',
	'1MA': '1 Maccabees',
	'1MACC': '1 Maccabees',
	'2MA': '2 Maccabees',
	'2MACC': '2 Maccabees',
	ESG: 'Esther (Greek)',
	ADDESTH: 'Esther (Greek)',
	DAG: 'Daniel (Greek)',
	'1ES': '1 Esdras',
	'1ESD': '1 Esdras',
	'2ES': '2 Esdras',
	'2ESD': '2 Esdras',
	MAN: 'Prayer of Manasseh',
	PRMAN: 'Prayer of Manasseh',
	LJE: 'Letter of Jeremiah',
	EPJER: 'Letter of Jeremiah',
	S3Y: 'Prayer of Azariah',
	PRAZAR: 'Prayer of Azariah',
	SUS: 'Susanna',
	BEL: 'Bel and the Dragon'
};

/** Books outside the canon by their usual names (USFM codes, OSIS ids). */
const bookName = (id: string) => DEUTERO[id.toUpperCase()] ?? id;

/** "A, B and C". */
function andList(items: string[]): string {
	return items.length < 2 ? (items[0] ?? '') : `${items.slice(0, -1).join(', ')} and ${items.at(-1)}`;
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

export function issueTitle(i: Issue): string {
	switch (i.kind) {
		case 'locked':
			return 'Module is locked';
		case 'failed':
			return 'Couldn’t read this file';
		case 'missing':
			if (i.books.length === 39) return 'No Old Testament in this file';
			if (i.books.length === 27) return 'No New Testament in this file';
			return `${plural(i.books.length, 'book', 'books')} not in this file`;
		case 'skipped': {
			const deutero = i.books.every((id) => bookName(id) !== id);
			return `${i.books.length} ${deutero ? 'deuterocanonical ' : ''}${i.books.length === 1 ? 'book' : 'books'} skipped`;
		}
		case 'warnings':
			return `${plural(i.list.length, 'verse', 'verses')} adjusted`;
	}
}

export function issueBadge(i: Issue): ['danger' | 'outline' | 'neutral', string] {
	switch (i.kind) {
		case 'locked':
			return ['danger', 'Needs action'];
		case 'failed':
			return ['danger', 'Failed'];
		case 'missing':
			return ['outline', 'Partial'];
		case 'skipped':
			return ['neutral', 'Skipped'];
		case 'warnings':
			return ['outline', 'Adjusted'];
	}
}

export function issueBody(i: Issue, names: string[]): string {
	switch (i.kind) {
		case 'locked':
			return 'The text is encrypted. Enter the unlock key from your purchase receipt.';
		case 'failed':
			return `${i.error.replace(/\.+$/, '')}.`;
		case 'missing': {
			const shown = i.books.slice(0, 4).map((b) => names[b] ?? '');
			const more = i.books.length - shown.length;
			return more > 0 ? `${shown.join(', ')} and ${more} more will be missing.` : `${andList(shown)} will be missing.`;
		}
		case 'skipped': {
			const list = i.books.map(bookName);
			return list.length === 1
				? `${list[0]} isn’t supported and was left out.`
				: `${andList(list)} aren’t supported and were left out.`;
		}
		case 'warnings':
			return i.list.length === 1 ? i.list[0] : `${i.list[0]} — and ${i.list.length - 1} more.`;
	}
}
