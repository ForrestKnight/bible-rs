<script lang="ts">
	import { resolve } from '$app/paths';
	import { NAME, REPO_URL } from './config.js';
	import { theme } from './theme.svelte.js';

	/** `ontoggle` runs after the theme changes, so the demo can click when its sound is on. */
	let { ontoggle }: { ontoggle?: () => void } = $props();

	function toggle() {
		theme.toggle();
		ontoggle?.();
	}
</script>

<nav class="nav">
	<div class="bar">
		<a class="brand" href="{resolve('/(site)')}#top" aria-label="{NAME}, home">
			<img src="/icon.png" width="26" height="26" alt="" />
			<span>{NAME}</span>
		</a>
		<div class="links">
			<a class="link" href="{resolve('/(site)')}#features">Features</a>
			<a class="link" href="{resolve('/(site)')}#shortcuts">Shortcuts</a>
			<a class="link" href="{resolve('/(site)')}#install">Install</a>
			<a class="link" href={REPO_URL}>GitHub</a>
			<button type="button" class="theme" aria-label="Light mode" aria-pressed={theme.current === 'light'} onclick={toggle}>
				<svg class="moon" viewBox="0 0 16 16" aria-hidden="true">
					<path d="M13.5 9.8A5.8 5.8 0 0 1 6.2 2.5a5.8 5.8 0 1 0 7.3 7.3z" />
				</svg>
				<svg class="sun" viewBox="0 0 16 16" aria-hidden="true">
					<circle cx="8" cy="8" r="3" />
					<path
						d="M8 1.5v1.6M8 12.9v1.6M1.5 8h1.6M12.9 8h1.6M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M3.4 12.6l1.1-1.1M11.5 4.5l1.1-1.1"
					/>
				</svg>
			</button>
			<a class="pill mac" href={resolve('/download')} data-sveltekit-reload>Download</a>
			<a class="pill web" href={resolve('/read')} data-sveltekit-reload>Read online</a>
		</div>
	</div>
</nav>

<style>
	.nav {
		position: sticky;
		top: 0;
		z-index: 10;
		max-width: 1200px;
		margin: 0 auto;
		padding: 8px 24px;
	}

	.bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 8px 8px 16px;
		border-radius: 999px;
		background: var(--glass);
		backdrop-filter: blur(20px) saturate(180%) brightness(106%);
		box-shadow: var(--glass-edge);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 9px;
		font: 600 15px var(--sans);
		letter-spacing: -0.01em;
		text-decoration: none;
	}

	.brand img {
		display: block;
	}

	.links {
		display: flex;
		align-items: center;
		gap: 22px;
	}

	.link {
		color: var(--muted);
		font-size: 14px;
		text-decoration: none;
		transition: color 0.15s;
	}

	.link:hover {
		color: var(--fg);
	}

	.theme {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		margin-inline: -6px;
		padding: 0;
		border: 0;
		border-radius: 999px;
		background: none;
		color: var(--muted);
		cursor: pointer;
		transition:
			color 0.15s,
			background-color 0.15s;
	}

	.theme:hover {
		background: color-mix(in srgb, var(--fg) 8%, transparent);
		color: var(--fg);
	}

	.theme svg {
		grid-area: 1 / 1;
		width: 16px;
		height: 16px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.4;
		stroke-linecap: round;
		stroke-linejoin: round;
		transition:
			opacity 0.25s,
			rotate 0.5s cubic-bezier(0.3, 1.4, 0.5, 1),
			scale 0.5s cubic-bezier(0.3, 1.4, 0.5, 1);
	}

	.sun,
	:global(:root[data-theme='light']) .moon {
		opacity: 0;
		rotate: -90deg;
		scale: 0.5;
	}

	:global(:root[data-theme='light']) .sun {
		opacity: 1;
		rotate: 0deg;
		scale: 1;
	}

	.pill {
		padding: 7px 14px;
		border-radius: 999px;
		background: var(--raised);
		box-shadow: var(--raised-edge);
		font: 500 14px var(--sans);
		text-decoration: none;
		transition: background-color 0.15s;
	}

	.pill:hover {
		background-color: color-mix(in srgb, var(--fg) 8%, transparent);
	}

	.web {
		display: none;
	}

	@media (max-width: 640px) {
		.links .link {
			display: none;
		}
	}

	/* Phones and tablets can't run the Mac app; send them to the web version. */
	@media (max-width: 640px), (hover: none) and (pointer: coarse) {
		.mac {
			display: none;
		}

		.web {
			display: inline;
		}
	}
</style>
