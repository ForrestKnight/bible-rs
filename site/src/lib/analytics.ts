import { dev } from '$app/env';

/**
 * Vercel Web Analytics as a plain <script> tag, for pages prerendered without JavaScript (csr =
 * false), where +layout.ts's `inject` never runs. Same script and endpoint as `inject` picks from
 * the base path Vercel sets at build time; nothing in development.
 */
export function analyticsTag(): string {
	if (dev) return '';
	const base: string | undefined = import.meta.env.VITE_VERCEL_OBSERVABILITY_BASEPATH;
	const src = base ? `${base}/insights/script.js` : '/_vercel/insights/script.js';
	const endpoint = base ? ` data-endpoint="${base}/insights"` : '';
	return `<script defer src="${src}"${endpoint} data-sdkn="@vercel/analytics/sveltekit"></script>`;
}
