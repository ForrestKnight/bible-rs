/**
 * The NLT's one chapter in the page (nlt.ts): asked of /api/nlt when the reader reaches it, held
 * in the core until the reader moves to another chapter or translation, and never stored. The
 * request skips the browser's cache, the response tells it to keep nothing, and the offline cache
 * (src/service-worker) leaves /api alone.
 */
import type { Core } from './core';
import { NLT_ID, nltReference } from './nlt';

/** Why a chapter didn't come: no connection, too many requests (try again in `retryAfter` s), or an error. */
export class LiveError extends Error {
	constructor(
		message: string,
		readonly kind: 'offline' | 'busy' | 'failed',
		readonly retryAfter = 0
	) {
		super(message);
	}
}

export class LiveChapter {
	/** The chapter held, once it has arrived. */
	held = $state<number | null>(null);
	#pending: { index: number; promise: Promise<boolean> } | null = null;
	#abort: AbortController | null = null;

	/**
	 * Fetches chapter `index` and holds it, letting the one before go first. Resolves true once it's
	 * held, false if another chapter was asked for meanwhile; throws a LiveError.
	 */
	read(core: Core, index: number): Promise<boolean> {
		if (this.held === index) return Promise.resolve(true);
		if (this.#pending?.index === index) return this.#pending.promise;
		this.drop(core);
		const abort = new AbortController();
		this.#abort = abort;
		const promise = this.#fetch(core, index, abort.signal).finally(() => {
			if (this.#pending?.promise === promise) this.#pending = null;
		});
		this.#pending = { index, promise };
		return promise;
	}

	/** Lets the chapter go (and stops one on its way). */
	drop(core: Core) {
		this.#abort?.abort();
		this.#abort = null;
		this.#pending = null;
		if (core.has(NLT_ID)) core.unload(NLT_ID);
		this.held = null;
	}

	async #fetch(core: Core, index: number, signal: AbortSignal): Promise<boolean> {
		const ref = encodeURIComponent(nltReference(core.books, index)).replace(/%20/g, '+');
		let res: Response;
		let body: { html?: unknown; error?: unknown } | null;
		try {
			res = await fetch(`/api/nlt?ref=${ref}`, { signal, cache: 'no-store' });
			body = await res.json().catch(() => null);
		} catch {
			if (signal.aborted) return false;
			throw new LiveError(navigator.onLine ? "couldn't reach the server" : "you're offline", 'offline');
		}
		if (signal.aborted) return false;
		const error = typeof body?.error === 'string' ? body.error : `error ${res.status}`;
		if (res.status === 429) throw new LiveError(error, 'busy', Number(res.headers.get('retry-after')) || 60);
		if (!res.ok || typeof body?.html !== 'string') throw new LiveError(error, 'failed');
		core.loadNlt(index, body.html);
		this.held = index;
		return true;
	}
}
