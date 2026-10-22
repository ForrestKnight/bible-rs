/**
 * The cited verses in the reader's current translation, for the viewer's hotspots and captions.
 * The bundled translations' text is prerendered into /<slug>/verses.json; the Mac app answers the
 * same address from its own library, for whatever translation it's set to; a translation imported
 * into this browser's reader is read through the core (imported.ts), which loads only then.
 */
import type { VerseTexts } from './spec';

/** The reader's settings (read/settings.ts SETTINGS_KEY; that module would bring the core along). */
const SETTINGS_KEY = 'bible.read.settings';

/** The Mac app's webview: wry gives the page `window.ipc`. */
export const inApp = () => typeof window !== 'undefined' && 'ipc' in window;

/** The translation the reader was last set to in this browser, if any. */
export function readerTranslation(): string | null {
	try {
		const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? '{}');
		return typeof s.translation === 'string' ? s.translation : null;
	} catch {
		return null;
	}
}

/** By address: the web's one file, or the app's answer for each translation it's asked for. */
const fetched = new Map<string, Promise<VerseTexts[]>>();

/** Every translation verses.json has (in the app: the app's translation, then the KJV). */
export function available(url: string, translation: string): Promise<VerseTexts[]> {
	const address = inApp() ? `${url}?translation=${encodeURIComponent(translation)}` : url;
	let list = fetched.get(address);
	if (!list) {
		list = fetch(address)
			.then((r) => (r.ok ? r.json() : { translations: [] }))
			.then((j: { translations: VerseTexts[] }) => j.translations)
			.catch(() => []);
		fetched.set(address, list);
	}
	return list;
}

/** Reads the cited verses from a translation imported into this browser's reader (imported.ts). */
export type ImportedTexts = (translation: string, refs: string[]) => Promise<VerseTexts | null>;

/**
 * The verses in `translation`, or null when neither verses.json nor `imported` has it. The page
 * passes `imported`; the app's embed doesn't, so its files never include the core.
 */
export async function textsIn(translation: string, url: string, refs: string[], imported?: ImportedTexts): Promise<VerseTexts | null> {
	const listed = (await available(url, translation)).find((t) => t.id === translation);
	if (listed || inApp() || !imported) return listed ?? null;
	return imported(translation, refs);
}
