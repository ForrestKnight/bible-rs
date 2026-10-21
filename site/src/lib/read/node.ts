/**
 * The core's files on disk, for prerendering (core.ts imports this only on the server). Paths are
 * relative to site/, where Vite runs; Vercel's build can read ../data because the project includes
 * files outside its root directory.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export const wasmBytes = () => readFileSync(resolve('src/lib/read/wasm/bible_bg.wasm'));

export function translationBytes(id: string): Uint8Array {
	if (!/^[a-z0-9-]+$/.test(id)) throw new Error(`no translation ${id}`);
	return readFileSync(resolve('../data/translations', `${id}.bbl`));
}
