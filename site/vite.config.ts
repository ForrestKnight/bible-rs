/// <reference types="vitest/config" />
import vercel from '@sveltejs/adapter-vercel';
import type { Adapter } from '@sveltejs/kit';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin, searchForWorkspaceRoot } from 'vite';
import { addHeaders, compactRoutes } from './scripts/vercel-routes.js';

/**
 * The object pages' 3D viewer, three.js with it: the chunks and styles that hold any of these.
 * The reader's service worker leaves them out of its offline copy (src/service-worker), so reading
 * never downloads a 3D engine. The client builds before the worker, which gets the list as
 * __VIEWER_FILES__.
 */
const VIEWER = /\/node_modules\/three\/|\/src\/lib\/objects\/(scene|cherub)\.ts$|\/src\/lib\/objects\/Viewer\.svelte$|\/src\/routes\/ark-of-the-covenant\//;

function viewerFiles(): Plugin {
	let files: string[] = [];
	return {
		name: 'viewer-files',
		generateBundle(_, bundle) {
			if (this.environment.name !== 'client') return;
			files = Object.values(bundle).flatMap((f) =>
				f.type === 'chunk' && f.moduleIds.some((id) => VIEWER.test(id))
					? [f.fileName, ...(f.viteMetadata?.importedCss ?? [])]
					: []
			);
		},
		transform(code) {
			if (this.environment.name !== 'serviceWorker' || !code.includes('__VIEWER_FILES__')) return;
			return code.replaceAll('__VIEWER_FILES__', JSON.stringify(files));
		}
	};
}

const base = vercel();
const adapter: Adapter = {
	...base,
	async adapt(builder) {
		await base.adapt(builder);
		compactRoutes('.vercel/output/config.json');
		// An NLT chapter's address shows the KJV's page (src/hooks.ts): kept out of search results.
		addHeaders('.vercel/output/config.json', '^/read/nlt(?:/.*)?$', { 'X-Robots-Tag': 'noindex' });
	}
};

export default defineConfig({
	plugins: [
		viewerFiles(),
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter,
			// The reader registers it (for /read only), so the landing page never downloads the reader.
			serviceWorker: { register: false }
		})
	],
	test: { include: ['src/**/*.test.ts'] },
	// The object pages import their specs from ../objects (objects/README.md).
	server: { fs: { allow: [searchForWorkspaceRoot(process.cwd()), '../objects'] } }
});
