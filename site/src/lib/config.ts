/**
 * Single source of truth for the download target and version shown on the page.
 * The page's buttons point at /download, which 302-redirects to DOWNLOAD_URL,
 * so the target can be changed here without touching the page.
 */
export const VERSION = '0.1.0';

/** `scripts/release-macos.sh` builds a universal app for macOS 11 and later. */
export const ARCH = 'Apple Silicon and Intel';
export const MIN_MACOS = '11';

/** The apps' name, on the Mac and iPhone, and in the site's headings and links that name them. */
export const NAME = 'Bible';
/**
 * The website's name. Every page's title ends with it, and it is og:site_name and the WebSite's
 * name in the JSON-LD. src/lib/topics/build.mjs reads it from here for the topic pages' titles.
 */
export const SITE_NAME = 'Simply My Bible';
export const REPO_URL = 'https://github.com/michaelshimeles/bible-rs';

/** The DMG on the latest GitHub release. */
export const DOWNLOAD_URL = `${REPO_URL}/releases/latest/download/bible-rs-universal.dmg`;

/**
 * Canonical site origin, no trailing slash. Every absolute URL in the page's metadata
 * is built from it; static/robots.txt repeats it by hand.
 */
export const SITE_URL = 'https://simplymybible.com';
