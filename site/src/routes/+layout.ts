import { dev } from '$app/env';
import { inject } from '@vercel/analytics';

// `@vercel/analytics/sveltekit` subscribes to `$app/stores`, which throws in SvelteKit 3. This
// passes the two things it would add, the script path and config Vercel sets at build time. The
// script then counts page views itself, on load and on each pushState.
inject(
	{
		mode: dev ? 'development' : 'production',
		framework: 'sveltekit',
		basePath: import.meta.env.VITE_VERCEL_OBSERVABILITY_BASEPATH
	},
	import.meta.env.VITE_VERCEL_OBSERVABILITY_CLIENT_CONFIG
);

export const prerender = true;
