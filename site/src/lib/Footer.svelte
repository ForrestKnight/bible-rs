<script lang="ts">
	import { resolve } from '$app/paths';
	import type { Snippet } from 'svelte';
	import { NAME, REPO_URL } from './config.js';

	let { children }: { children?: Snippet } = $props();
</script>

<footer class="footer">
	<div class="brand">
		<img src="/icon.png" width="20" height="20" alt="" />
		<span>{NAME}</span>
	</div>
	<p>Made by <a href="https://rasmic.xyz">Micky</a>. {@render children?.()}</p>
	<div class="links">
		<a href={resolve('/(site)/privacy')}>Privacy</a>
		<a href={resolve('/(site)/support')}>Support</a>
		<a href={REPO_URL}>
			<svg viewBox="0 0 16 16" aria-hidden="true">
				<path
					d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"
				/>
			</svg>
			GitHub
		</a>
	</div>
</footer>

<style>
	.footer {
		display: flex;
		align-items: center;
		gap: 24px;
		max-width: 1080px;
		margin: 0 auto;
		padding: 32px 24px 48px;
		border-top: 1px solid var(--line);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 9px;
		font: 600 14px var(--sans);
	}

	.brand img {
		display: block;
	}

	p {
		flex: 1;
		margin: 0;
		color: var(--muted);
		font-size: 13px;
	}

	.links {
		display: flex;
		gap: 20px;
	}

	.links a {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--muted);
		font-size: 13px;
		text-decoration: none;
		transition: color 0.15s;
	}

	.links a:hover {
		color: var(--fg);
	}

	svg {
		width: 15px;
		height: 15px;
		fill: currentColor;
	}

	@media (max-width: 640px) {
		.footer {
			flex-wrap: wrap;
			gap: 12px 24px;
		}

		p {
			flex-basis: 100%;
			order: 3;
		}
	}
</style>
