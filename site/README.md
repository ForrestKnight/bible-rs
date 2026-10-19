# Bible lander (SvelteKit)

- Download target + version: `src/lib/config.ts` (`DOWNLOAD_URL`, `VERSION`, `SITE_URL`).
- `/download` returns a 302 to `DOWNLOAD_URL` (serverless function); the page itself is prerendered.
- Page metadata (title, canonical, Open Graph, Twitter, JSON-LD) lives in `src/routes/+page.svelte` and is built from `SITE_URL`. `static/robots.txt` and `static/sitemap.xml` repeat the origin.
- Requires Node >= 22.17 (SvelteKit 3). `.npmrc` sets `engine-strict`, so on a newer Node use `npm install --engine-strict=false`.
- `npm install && npm run build && npx vite preview`
- Share image: `static/og.jpg` (1200x630) is rendered from `scripts/og/og.html`. Edit the template, then run `sh scripts/og.sh` (macOS, needs Google Chrome).
- Vercel: project `bible-rs`, team `goshen-labs`, builds from `site/` -> https://bible.rasmic.xyz
