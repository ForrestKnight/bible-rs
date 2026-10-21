/** Text broken into lines as the Mac's `flow` does: greedy, at most `max`, the last elided. */

let ctx: CanvasRenderingContext2D | null = null;

export function lines(text: string, font: string, width: number, max: number): string[] {
	ctx ??= document.createElement('canvas').getContext('2d');
	if (!ctx) return [text];
	const c = ctx;
	c.font = font;
	const fits = (s: string) => c.measureText(s).width <= width;
	const words = text.split(/\s+/).filter(Boolean);
	const out: string[] = [];
	let line = '';
	for (let i = 0; i < words.length; i++) {
		const next = line ? `${line} ${words[i]}` : words[i];
		if (!line || fits(next)) {
			line = next;
			continue;
		}
		if (out.length === max - 1) {
			let last = `${line} ${words.slice(i).join(' ')}`;
			while (last.length > 1 && !fits(`${last}…`)) last = last.slice(0, -1).trimEnd();
			out.push(`${last}…`);
			return out;
		}
		out.push(line);
		line = words[i];
	}
	if (line) out.push(line);
	return out;
}
