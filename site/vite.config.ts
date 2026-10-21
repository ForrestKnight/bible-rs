import vercel from '@sveltejs/adapter-vercel';
import type { Adapter } from '@sveltejs/kit';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { compactRoutes } from './scripts/vercel-routes.js';

const base = vercel();
const adapter: Adapter = {
	...base,
	async adapt(builder) {
		await base.adapt(builder);
		compactRoutes('.vercel/output/config.json');
	}
};

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter,
			// The reader registers it (for /read only), so the landing page never downloads the reader.
			serviceWorker: { register: false }
		})
	]
});
