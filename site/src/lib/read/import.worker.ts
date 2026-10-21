/// <reference lib="webworker" />
/**
 * Converts a translation the reader brings, off the page's thread: bible-core's importers in their
 * own wasm module (scripts/build-web.sh), which only this worker loads. See imports.ts.
 */
import init, { importTranslation, peekImport } from './wasm/bible_import.js';
import wasmUrl from './wasm/bible_import_bg.wasm?url';
import type { ImportOptions, ImportProgress, PickedFile } from './imports';

declare const self: DedicatedWorkerGlobalScope;

type Request =
	| { id: number; type: 'peek'; label: string; files: PickedFile[] }
	| { id: number; type: 'import'; label: string; files: PickedFile[]; options: ImportOptions; taken: string[] };

const ready = init({ module_or_path: wasmUrl });

async function read(files: PickedFile[]): Promise<[string[], Uint8Array[]]> {
	const contents = await Promise.all(files.map(async (f) => new Uint8Array(await f.file.arrayBuffer())));
	return [files.map((f) => f.path), contents];
}

self.onmessage = async (e: MessageEvent<Request>) => {
	const m = e.data;
	const progress = (p: Partial<ImportProgress>) => self.postMessage({ id: m.id, progress: p });
	try {
		await ready;
		if (m.type === 'peek') {
			const [paths, contents] = await read(m.files);
			self.postMessage({ id: m.id, ok: true, value: peekImport(paths, contents, m.label) });
			return;
		}
		progress({ step: 0 });
		const [paths, contents] = await read(m.files);
		progress({ step: 1 });
		const value = importTranslation(paths, contents, m.label, m.options, m.taken, (books: number, book: number) =>
			progress({ step: 2, books, book })
		);
		self.postMessage({ id: m.id, ok: true, value }, [value.bytes.buffer]);
	} catch (err) {
		self.postMessage({ id: m.id, ok: false, error: err instanceof Error ? err.message : String(err) });
	}
};
