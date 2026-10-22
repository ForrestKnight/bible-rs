import type { Reroute } from '@sveltejs/kit/hooks';

/**
 * An NLT chapter's address (/read/nlt/john/3) shows the same chapter's KJV page, prerendered with
 * its canonical address and description, and the reader then fetches the NLT (lib/read/nlt.ts):
 * no NLT text is ever in a page. Vercel adds `X-Robots-Tag: noindex` to these addresses
 * (vite.config.ts) and the reader its meta tag while it shows the NLT.
 */
export const reroute: Reroute = ({ url }) => {
	const m = /^\/read\/nlt(\/[^/]+\/[^/]+\/?)$/.exec(url.pathname);
	return m ? `/read/kjv${m[1]}` : undefined;
};
