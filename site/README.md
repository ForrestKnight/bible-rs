# bible-rs lander (SvelteKit)

- Download target + version: `src/lib/config.ts` (`DOWNLOAD_URL`, `VERSION`, `SITE_URL`).
- `/download` returns a 302 to `DOWNLOAD_URL` (serverless function); the page itself is prerendered.
- Requires Node >= 22.17 (SvelteKit 3). On this box: `export PATH=/home/box/.local/node22/bin:$PATH`.
- `npm install && npm run build && npx vite preview`
- Vercel: project `bible-rs`, team `goshen-labs` -> https://bible-rs.vercel.app
