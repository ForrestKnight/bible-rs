export const clamp = (x: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));

export const lerp = (a: number, b: number, p: number) => a + (b - a) * p;

/** Progress of `t` through the window [start, end], clamped to 0..1. */
export const span = (t: number, start: number, end: number) => clamp((t - start) / (end - start));

export const easeOut = (p: number) => 1 - (1 - p) ** 3;

export const easeInOut = (p: number) => (p < 0.5 ? 4 * p ** 3 : 1 - (-2 * p + 2) ** 3 / 2);

/** Ease out with a slight overshoot, for things that pop into place. */
export const easeOutBack = (p: number) => {
	const k = 1.6;
	return 1 + (k + 1) * (p - 1) ** 3 + k * (p - 1) ** 2;
};

/** 0 → 1 over [inStart, inEnd], then 1 → 0 over [outStart, outEnd]. */
export const envelope = (t: number, inStart: number, inEnd: number, outStart: number, outEnd: number) =>
	Math.min(easeOut(span(t, inStart, inEnd)), 1 - easeInOut(span(t, outStart, outEnd)));
