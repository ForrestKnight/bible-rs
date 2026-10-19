/**
 * Single source of truth for the download target and version shown on the page.
 * The page's button points at /download, which 302-redirects to DOWNLOAD_URL,
 * so the target can be changed here without touching the page.
 */
export const VERSION = '0.1.0';

/** Public repo michaelshimeles/bible-rs: GitHub "latest release" asset URL. */
export const DOWNLOAD_URL =
	'https://github.com/michaelshimeles/bible-rs/releases/latest/download/bible-rs-universal.dmg';

/** Canonical site origin, used for absolute og:image / og:url. */
export const SITE_URL = 'https://bible-rs.vercel.app';
