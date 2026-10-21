# Bible site (SvelteKit)

The landing page, privacy and support pages (`src/routes/(site)`), and the web reader at `/read`.

## Landing page

- Download target + version: `src/lib/config.ts` (`DOWNLOAD_URL`, `VERSION`, `SITE_URL`).
- `/download` returns a 302 to `DOWNLOAD_URL` (serverless function); the page itself is prerendered.
- Page metadata (title, canonical, Open Graph, Twitter, JSON-LD) lives in `src/routes/(site)/+page.svelte` and is built from `SITE_URL`. `static/robots.txt` repeats the origin; `/sitemap.xml` is generated (`src/routes/sitemap.xml`) and lists every reader chapter.
- The demo under the hero is drawn in HTML, not recorded: `src/lib/demo/script.ts` is its timeline (chapters, captions, sound cues), `Scene.svelte` renders one moment of it, `Showcase.svelte` adds the controls and `sound.ts` synthesises the clicks with Web Audio.
- The feature cards are in `src/lib/cards/`, built from the app's UI pieces in `src/lib/app/` and the app's colours in `src/app.css` (`.app`).
- Scripture shown on the site lives in `src/lib/scripture.ts`. Use only public-domain text (KJV, WEB, BSB and the other built-in translations), never the online ones.
- Fonts: system fonts for the UI, and Literata for scripture, self-hosted from `static/fonts` (OFL). The only other origin the site loads from is the GitHub API, for the download count under the hero's button (`src/lib/DownloadCount.svelte`).
- Share image: `static/og.jpg` (1200x630) is rendered from `scripts/og/og.html`. Edit the template, then run `sh scripts/og.sh` (macOS, needs Google Chrome).

## Reader (`/read`)

The Mac app in the browser. [DECISIONS.md](../DECISIONS.md#web-the-core-in-webassembly-the-reader-in-the-dom) explains the design.

- `src/lib/read/wasm/` is bible-core compiled to WebAssembly by `scripts/build-web.sh` (from the repository root): `bible` for the reader and `bible_import` (with the importers) for the import worker. Both are committed; rebuild them after changing `crates/web` or what it uses of `crates/core`.
- Importing: `src/lib/read/imports.ts` picks or takes dropped files, talks to `import.worker.ts` (which runs `bible_import`), and stores imported translations in IndexedDB (`bible-read`) with their list in `localStorage`. `overlays/importer.svelte.ts` is the import sheet's state.
- `/read/onboarding.json` (prerendered) holds the first-visit screen's translation previews. The first visit shows it unless the address names a translation.
- `src/lib/read/core.ts` wraps the module: on the server it reads the wasm and `data/translations/*.bbl` from disk (`node.ts`); in the browser it fetches them (`/translations/<id>.bbl`, prerendered by `src/routes/translations`).
- Routes: `src/routes/read/+layout.server.ts` typesets the chapter with the default settings for every page; `/read/[translation]/[book]/[chapter]` is prerendered for every chapter of every bundled translation; `/read` restores the saved place.
- `src/lib/read/app.svelte.ts` is the app's state and actions (the Mac's `App`); `Shell.svelte` is the window; `keyboard.ts` the shortcuts; `reader/` the continuous-scroll reader; the rest mirrors the Mac's components (`chrome/`, `sidebar/`, `overlays/`, `pages/`, `settings/`, `onboarding/`), drawn with the controls in `ui/`.
- Settings are the Mac's `Settings` (`settings.ts`), saved in `localStorage` with the study data (the core's JSON export) and up to a few automatic backups.
- Design tokens: `src/lib/read/tokens.css` is generated from `crates/app/assets/design-tokens.json` by `node scripts/tokens.js`. Fonts: `static/fonts/read/` is cut from the Mac app's Literata and Inter by `scripts/fonts.sh` (needs fonttools and brotli). Both outputs are committed.
- `src/service-worker/` caches the reader for offline use. It is registered by the reader only, scoped to `/read`, and not in development.
- Development: `npm run dev`, then <http://localhost:5173/read>. In development the reader takes the Mac's demo steps for screenshots, for example `/read/kjv/john/1?fixture&settings={"theme":"dark"}&demo=jump:jn 3:16;select:jn 3:16` (`src/lib/read/demo.ts`): `fixture` loads sample highlights, notes and bookmarks, `settings` overrides settings, and `demo` runs steps in order. `window.readApp` is the app.

## Building

- Requires Node >= 22.17 (SvelteKit 3). `.npmrc` sets `engine-strict`, so on a newer Node use `npm install --engine-strict=false`.
- `npm install && npm run build && npx vite preview`. The build prerenders the reader's 8,323 chapter pages.
- Vercel: project `bible-rs`, team `goshen-labs`, builds from `site/` -> https://bible.rasmic.xyz. The build reads `../data/translations`, so the project includes files outside its root directory.
