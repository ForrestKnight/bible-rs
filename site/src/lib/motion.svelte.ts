import type { Attachment } from 'svelte/attachments';
import { MediaQuery } from 'svelte/reactivity';
import { onFrame } from './frame.js';

export const reducedMotion = new MediaQuery('(prefers-reduced-motion: reduce)', false);

/** Reports whether at least `threshold` of the element is on screen in a visible tab. */
function watchVisibility(node: Element, onChange: (visible: boolean) => void, threshold = 0.35) {
	let intersecting = false;
	const update = () => onChange(intersecting && !document.hidden);
	const observer = new IntersectionObserver(
		([entry]) => {
			intersecting = entry.isIntersecting;
			update();
		},
		{ threshold }
	);
	observer.observe(node);
	document.addEventListener('visibilitychange', update);
	return () => {
		observer.disconnect();
		document.removeEventListener('visibilitychange', update);
	};
}

/** Calls `tick` every frame with the milliseconds since the last one, while `node` is on screen. */
export function whileVisible(node: Element, tick: (dt: number) => void) {
	let stop: (() => void) | undefined;
	const unwatch = watchVisibility(node, (visible) => {
		stop?.();
		stop = undefined;
		if (!visible) return;
		let last = performance.now();
		stop = onFrame((now) => {
			tick(Math.min(100, now - last));
			last = now;
		});
	});
	return () => {
		unwatch();
		stop?.();
	};
}

/**
 * A looping clock in milliseconds that only runs while its element is visible. With reduced motion
 * it stays on `settled`, a moment where the loop shows everything it animates.
 */
export function loop(duration: number, settled: number) {
	let elapsed = $state(0);

	const observe: Attachment<HTMLElement> = (node) => {
		if (reducedMotion.current) return;
		return whileVisible(node, (dt) => (elapsed = (elapsed + dt) % duration));
	};

	return {
		get t() {
			return reducedMotion.current ? settled : elapsed;
		},
		observe
	};
}

/**
 * Fades and lifts an element in the first time it scrolls into view. Elements start visible and are
 * only hidden here, so the page still reads without JavaScript. Teardown shows the element, since
 * turning on reduced motion reruns this without the observer that would have.
 */
export const reveal: Attachment<HTMLElement> = (node) => {
	if (reducedMotion.current || node.getBoundingClientRect().top < innerHeight) return;
	node.dataset.reveal = '';
	const observer = new IntersectionObserver(
		([entry]) => {
			if (!entry.isIntersecting) return;
			node.dataset.shown = '';
			observer.disconnect();
		},
		{ threshold: 0.15, rootMargin: '0px 0px -40px 0px' }
	);
	observer.observe(node);
	return () => {
		observer.disconnect();
		node.dataset.shown = '';
	};
};
