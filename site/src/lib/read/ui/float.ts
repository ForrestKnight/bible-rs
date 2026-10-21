/**
 * Placing popovers and menus beside what they open from, kept inside the window, and moving them
 * out to <body> so a backdrop filter or transform on an ancestor can't trap or clip them.
 */
export type Side = 'bottom' | 'top' | 'right' | 'left';
export type Align = 'start' | 'center' | 'end';

export interface Placement {
	x: number;
	y: number;
	side: Side;
}

/** Room kept between a popover and the window's edges. */
const MARGIN = 8;

const opposite: Record<Side, Side> = { bottom: 'top', top: 'bottom', right: 'left', left: 'right' };

/**
 * Where a `w` × `h` panel goes on `side` of `anchor`, `gap` away, aligned to its start, centre or
 * end; flipped to the other side when it doesn't fit (unless `flip` is off), then nudged inside
 * the viewport.
 */
export function place(
	anchor: DOMRect,
	w: number,
	h: number,
	side: Side,
	align: Align,
	gap: number,
	flip = true
): Placement {
	const vw = window.innerWidth;
	const vh = window.innerHeight;
	const fits = (s: Side) =>
		s === 'bottom'
			? anchor.bottom + gap + h <= vh - MARGIN
			: s === 'top'
				? anchor.top - gap - h >= MARGIN
				: s === 'right'
					? anchor.right + gap + w <= vw - MARGIN
					: anchor.left - gap - w >= MARGIN;
	const s = !flip || fits(side) || !fits(opposite[side]) ? side : opposite[side];
	let x: number;
	let y: number;
	if (s === 'bottom' || s === 'top') {
		y = s === 'bottom' ? anchor.bottom + gap : anchor.top - gap - h;
		x = align === 'start' ? anchor.left : align === 'end' ? anchor.right - w : anchor.left + anchor.width / 2 - w / 2;
	} else {
		x = s === 'right' ? anchor.right + gap : anchor.left - gap - w;
		y = align === 'start' ? anchor.top : align === 'end' ? anchor.bottom - h : anchor.top + anchor.height / 2 - h / 2;
	}
	x = Math.max(MARGIN, Math.min(x, vw - w - MARGIN));
	y = Math.max(MARGIN, Math.min(y, vh - h - MARGIN));
	return { x: Math.round(x), y: Math.round(y), side: s };
}

/** A point as a zero-size rectangle (context menus open at the pointer). */
export const pointRect = (x: number, y: number) => new DOMRect(x, y, 0, 0);

/** Moves the element to the end of <body> while it's mounted. */
export function portal(node: HTMLElement) {
	document.body.appendChild(node);
	return {
		destroy() {
			node.remove();
		}
	};
}

const FOCUSABLE =
	'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export const focusables = (root: HTMLElement) =>
	[...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => !el.closest('[inert]') && el.offsetParent !== null);

/** Keeps Tab inside `root` (a modal's own controls). */
export function trapTab(root: HTMLElement, e: KeyboardEvent) {
	if (e.key !== 'Tab') return;
	const els = focusables(root);
	if (!els.length) {
		e.preventDefault();
		return;
	}
	const first = els[0];
	const last = els[els.length - 1];
	const active = document.activeElement;
	if (e.shiftKey && (active === first || !root.contains(active))) {
		e.preventDefault();
		last.focus();
	} else if (!e.shiftKey && (active === last || !root.contains(active))) {
		e.preventDefault();
		first.focus();
	}
}
