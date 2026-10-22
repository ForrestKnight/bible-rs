import type { Action } from 'svelte/action';

type Go = (e: MouseEvent) => void;

/**
 * A link the reader follows itself (`<a class="follow" href=… use:follow={go}>`), so a place can be
 * a real link (for search engines, a new tab, Copy Link) and still act like the button it replaced:
 * a plain click, return, or Space when a focused button would take it, runs `go` (keyboard presses
 * arrive with `detail` 0, as on a button); a click with a modifier or another button stays the
 * browser's. It listens on the link itself, so it runs before SvelteKit's router (on <html>) even
 * when a popover has moved the link out of the app's root.
 */
export const follow: Action<HTMLAnchorElement, Go> = (a, go) => {
	let run = go;
	let spaced = false;
	a.draggable = false;
	// Nothing to preload: the reader moves to the place without loading the page.
	a.dataset.sveltekitPreloadData = 'false';

	const click = (e: MouseEvent) => {
		if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
		e.preventDefault();
		run(e);
	};
	// keyboard.ts `pressable`: Space presses a control in a popover or one reached by keyboard;
	// otherwise it's left to scroll the reader.
	const keydown = (e: KeyboardEvent) => {
		if (e.key !== ' ' || !(a.closest('.popover') || a.matches(':focus-visible'))) return;
		e.preventDefault();
		spaced = true;
	};
	const keyup = (e: KeyboardEvent) => {
		if (e.key !== ' ' || !spaced) return;
		spaced = false;
		a.click();
	};
	a.addEventListener('click', click);
	a.addEventListener('keydown', keydown);
	a.addEventListener('keyup', keyup);
	return {
		update: (next) => (run = next),
		destroy() {
			a.removeEventListener('click', click);
			a.removeEventListener('keydown', keydown);
			a.removeEventListener('keyup', keyup);
		}
	};
};
