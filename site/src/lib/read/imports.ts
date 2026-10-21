/**
 * Translations imported in this browser: their `.bbl` files in IndexedDB, their metadata in
 * localStorage (so the library knows them before anything async runs), and the worker that
 * converts what the reader picks (import.worker.ts, with the importers' own wasm module).
 */
import type { TranslationInfo } from './core';

export const IMPORTED_KEY = 'bible.read.imported';
const DB = 'bible-read';
const STORE = 'translations';

/** A picked file, by its path inside what was picked. */
export interface PickedFile {
	path: string;
	file: Blob;
}

/** What was picked or dropped: a file, a folder's files, or a zip. */
export interface Picked {
	label: string;
	files: PickedFile[];
}

/** What a source holds, read before converting it (bible-core's `Peek`). */
export interface Peek {
	format: string;
	sword: boolean;
	label: string;
	bytes: number;
	abbrev: string;
	name: string;
	locked: boolean;
	/** "zText", "KJV versification". */
	details: string[];
}

export interface ImportResult {
	info: TranslationInfo;
	format: string;
	verses: number;
	/** Books of the 66 the source didn't have. */
	missing: number[];
	/** Books outside the canon, left out ("TOB"). */
	skippedBooks: string[];
	warnings: string[];
	/** The `.bbl` file. */
	bytes: Uint8Array;
}

export interface ImportOptions {
	abbrev?: string;
	name?: string;
	unlockKey?: string;
}

/** Progress from the worker: the step (0 reading, 1 detecting, 2 converting, 3 checking) and books. */
export interface ImportProgress {
	step: number;
	books: number;
	/** The book being converted (0–65), or -1. */
	book: number;
}

// ---------------------------------------------------------------- storage

export function importedInfos(): TranslationInfo[] {
	try {
		const list = JSON.parse(localStorage.getItem(IMPORTED_KEY) ?? '[]');
		return Array.isArray(list) ? list.filter((t) => t && typeof t.id === 'string' && t.imported) : [];
	} catch {
		return [];
	}
}

function saveInfos(list: TranslationInfo[]) {
	localStorage.setItem(IMPORTED_KEY, JSON.stringify(list));
}

let db: Promise<IDBDatabase> | undefined;

function open(): Promise<IDBDatabase> {
	db ??= new Promise((resolve, reject) => {
		const req = indexedDB.open(DB, 1);
		req.onupgradeneeded = () => req.result.createObjectStore(STORE);
		req.onsuccess = () => resolve(req.result);
		req.onerror = () => reject(req.error);
	});
	db.catch(() => (db = undefined));
	return db;
}

/** Runs one request, settling once its transaction has committed (or failed). */
async function run<T>(mode: IDBTransactionMode, op: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
	const tx = (await open()).transaction(STORE, mode);
	return new Promise((resolve, reject) => {
		const req = op(tx.objectStore(STORE));
		tx.oncomplete = () => resolve(req.result);
		tx.onabort = tx.onerror = () => reject(tx.error ?? req.error ?? new Error('the browser stopped saving it'));
	});
}

/** An imported translation's file. */
export async function importedBytes(id: string): Promise<Uint8Array> {
	const bytes = await run<Uint8Array | undefined>('readonly', (s) => s.get(id));
	if (!bytes) throw new Error('its file is missing from this browser');
	return bytes;
}

/** Keeps an imported translation: the file, then its entry in the list. */
export async function storeImported(info: TranslationInfo, bytes: Uint8Array): Promise<void> {
	await run('readwrite', (s) => s.put(bytes, info.id));
	saveInfos([...importedInfos().filter((t) => t.id !== info.id), info]);
}

export async function deleteImported(id: string): Promise<void> {
	saveInfos(importedInfos().filter((t) => t.id !== id));
	await run('readwrite', (s) => s.delete(id));
}

// ---------------------------------------------------------------- the worker

type Reply =
	| { id: number; progress: Partial<ImportProgress> }
	| { id: number; ok: true; value: unknown }
	| { id: number; ok: false; error: string };

let worker: Worker | null = null;
let nextId = 0;
const pending = new Map<
	number,
	{ resolve: (v: unknown) => void; reject: (e: Error) => void; progress?: (p: Partial<ImportProgress>) => void }
>();

function call<T>(message: object, progress?: (p: Partial<ImportProgress>) => void): Promise<T> {
	if (!worker) {
		worker = new Worker(new URL('./import.worker.ts', import.meta.url), { type: 'module' });
		worker.onmessage = (e: MessageEvent<Reply>) => {
			const r = e.data;
			const p = pending.get(r.id);
			if (!p) return;
			if ('progress' in r) p.progress?.(r.progress);
			else {
				pending.delete(r.id);
				if (r.ok) p.resolve(r.value);
				else p.reject(new Error(r.error));
			}
		};
		worker.onerror = (e) => {
			e.preventDefault();
			reset(new Error(e.message || "the importer couldn't start"));
		};
		worker.onmessageerror = () => reset(new Error("the importer's reply couldn't be read"));
	}
	const id = nextId++;
	return new Promise<T>((resolve, reject) => {
		pending.set(id, { resolve: resolve as (v: unknown) => void, reject, progress });
		worker!.postMessage({ id, ...message });
	});
}

export const peekSource = (picked: Picked) => call<Peek>({ type: 'peek', ...picked });

export const convertSource = (
	picked: Picked,
	options: ImportOptions,
	taken: string[],
	progress: (p: Partial<ImportProgress>) => void
) => call<ImportResult>({ type: 'import', ...picked, options, taken }, progress);

/** Stops a conversion under way: the worker goes, and the next call starts another. */
export function cancelConversion() {
	reset(new Error('import cancelled'));
}

function reset(error: Error) {
	worker?.terminate();
	worker = null;
	for (const p of pending.values()) p.reject(error);
	pending.clear();
}

// ---------------------------------------------------------------- picking

/** Asks for a file (or a folder's files). */
export function pickSource(folder: boolean): Promise<Picked | null> {
	return new Promise((resolve) => {
		const input = document.createElement('input');
		input.type = 'file';
		if (folder) input.webkitdirectory = true;
		else input.accept = '.xml,.osis,.usfm,.sfm,.ptx,.zip,.txt';
		input.onchange = () => resolve(input.files?.length ? fromFiles([...input.files]) : null);
		input.oncancel = () => resolve(null);
		input.click();
	});
}

/** Picked files: one file as itself; a folder's (relative paths) inside its name. */
function fromFiles(files: File[]): Picked {
	const rel = files[0].webkitRelativePath;
	if (!rel) return { label: files[0].name, files: [{ path: files[0].name, file: files[0] }] };
	const label = rel.split('/')[0];
	return {
		label,
		files: files.map((f) => ({ path: f.webkitRelativePath.split('/').slice(1).join('/'), file: f }))
	};
}

/** What was dropped: the first item, walking a folder. */
export async function fromDrop(data: DataTransfer): Promise<Picked | null> {
	const entry = [...data.items].find((i) => i.kind === 'file')?.webkitGetAsEntry();
	if (!entry) return data.files.length ? fromFiles([data.files[0]]) : null;
	if (entry.isFile) {
		const file = await new Promise<File>((res, rej) => (entry as FileSystemFileEntry).file(res, rej));
		return { label: file.name, files: [{ path: file.name, file }] };
	}
	const files: PickedFile[] = [];
	await walk(entry as FileSystemDirectoryEntry, '', files);
	return { label: entry.name, files };
}

async function walk(dir: FileSystemDirectoryEntry, prefix: string, out: PickedFile[]) {
	const reader = dir.createReader();
	for (;;) {
		const batch = await new Promise<FileSystemEntry[]>((res, rej) => reader.readEntries(res, rej));
		if (!batch.length) break;
		for (const e of batch) {
			const path = prefix + e.name;
			if (e.isDirectory) await walk(e as FileSystemDirectoryEntry, `${path}/`, out);
			else out.push({ path, file: await new Promise<File>((res, rej) => (e as FileSystemFileEntry).file(res, rej)) });
		}
	}
}

/** Whether a drag carries files (their names come only with the drop). */
export const dragsFiles = (data: DataTransfer | null) => !!data && [...data.types].includes('Files');
