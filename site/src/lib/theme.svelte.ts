import { browser } from '$app/env';
import { reducedMotion } from './motion.svelte.js';

type Theme = 'dark' | 'light';

/**
 * The page theme, stored on `<html data-theme>`. app.html restores the saved choice before the
 * first paint, so everything that depends on it is styled from that attribute in CSS.
 */
class PageTheme {
	current = $state<Theme>(browser && document.documentElement.dataset.theme === 'light' ? 'light' : 'dark');

	toggle() {
		const next = this.current === 'dark' ? 'light' : 'dark';
		const apply = () => {
			this.current = next;
			const root = document.documentElement;
			root.dataset.theme = next;
			const bg = getComputedStyle(root).getPropertyValue('--bg').trim();
			document.querySelector('meta[name="theme-color"]')?.setAttribute('content', bg);
		};
		if (document.startViewTransition && !reducedMotion.current) document.startViewTransition(apply);
		else apply();
		try {
			localStorage.setItem('theme', next);
		} catch {
			// Storage can be blocked; the theme still applies for this visit.
		}
	}
}

export const theme = new PageTheme();
