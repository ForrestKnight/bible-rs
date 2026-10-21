/** Local dates and times for history and lists: "2 h ago", "Wed", "9:41" (the Mac's time.rs). */
import { now } from './app.svelte';

const WEEKDAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

const date = (unix: number) => new Date(unix * 1000);

export const weekday = (unix: number) => WEEKDAYS[date(unix).getDay()];
export const weekdayShort = (unix: number) => weekday(unix).slice(0, 3);

/** Local midnight at the start of the day containing `unix`. */
export function dayStart(unix: number): number {
	const d = date(unix);
	d.setHours(0, 0, 0, 0);
	return Math.floor(d.getTime() / 1000);
}

/** "9:41" (a 24-hour clock, as the design shows). */
export function clock(unix: number): string {
	const d = date(unix);
	return `${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** "Oct 3". */
export function shortDate(unix: number): string {
	const d = date(unix);
	return `${MONTHS[d.getMonth()]} ${d.getDate()}`;
}

/** How long ago, briefly: "Just now", "12 min ago", "2 h ago", "Wed", "Oct 3". */
export function ago(unix: number): string {
	const t = now();
	const secs = Math.max(0, t - unix);
	if (secs < 60) return 'Just now';
	if (secs < 3600) return `${Math.floor(secs / 60)} min ago`;
	if (unix >= dayStart(t)) return `${Math.floor(secs / 3600)} h ago`;
	if (unix >= dayStart(t) - 6 * 86_400) return weekdayShort(unix);
	return shortDate(unix);
}

/** "12 min", "1 h 56 m". */
export function duration(seconds: number): string {
	const m = Math.floor((seconds + 30) / 60);
	return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60} m`;
}
