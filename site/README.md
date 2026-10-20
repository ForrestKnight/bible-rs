# Bible lander (SvelteKit)

- Download target + version: `src/lib/config.ts` (`DOWNLOAD_URL`, `VERSION`, `SITE_URL`).
- `/download` returns a 302 to `DOWNLOAD_URL` (serverless function); the page itself is prerendered.
- Page metadata (title, canonical, Open Graph, Twitter, JSON-LD) lives in `src/routes/+page.svelte` and is built from `SITE_URL`. `static/robots.txt` and `static/sitemap.xml` repeat the origin.
- The demo under the hero is drawn in HTML, not recorded: `src/lib/demo/script.ts` is its timeline (chapters, captions, sound cues), `Scene.svelte` renders one moment of it, `Showcase.svelte` adds the controls and `sound.ts` synthesises the clicks with Web Audio.
- The feature cards are in `src/lib/cards/`, built from the app's UI pieces in `src/lib/app/` and the app's colours in `src/app.css` (`.app`).
- Scripture shown on the site lives in `src/lib/scripture.ts`. Use only public-domain text (KJV, WEB, BSB and the other built-in translations), never the online ones.
- Fonts: system fonts for the UI, and Literata for scripture, self-hosted from `static/fonts` (OFL). The only other origin the site loads from is the GitHub API, for the download count under the hero's button (`src/lib/DownloadCount.svelte`).
- Requires Node >= 22.17 (SvelteKit 3). `.npmrc` sets `engine-strict`, so on a newer Node use `npm install --engine-strict=false`.
- `npm install && npm run build && npx vite preview`
- Share image: `static/og.jpg` (1200x630) is rendered from `scripts/og/og.html`. Edit the template, then run `sh scripts/og.sh` (macOS, needs Google Chrome).
- Vercel: project `bible-rs`, team `goshen-labs`, builds from `site/` -> https://bible.rasmic.xyz
