type Tick = (now: number) => void;
const ticks = new Set<Tick>();
let frame = 0;

function run(now: number) {
	for (const tick of ticks) tick(now);
	frame = ticks.size ? requestAnimationFrame(run) : 0;
}

/** Calls `tick` on every animation frame, sharing one requestAnimationFrame loop across the page. */
export function onFrame(tick: Tick) {
	ticks.add(tick);
	frame ||= requestAnimationFrame(run);
	return () => {
		ticks.delete(tick);
	};
}
