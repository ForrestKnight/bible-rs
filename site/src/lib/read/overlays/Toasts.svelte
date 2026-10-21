<!--
	Toasts: inverse capsules at the bottom centre of the reading area, the newest on top, then the
	task under way. A failure that can be retried stays until it's dismissed (the Mac's toasts).
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Icon } from '../ui';

	const app = getApp();
</script>

<div class="toasts" role="status" aria-live="polite">
	{#each app.toasts as t (t.id)}
		{#if t.retry}
			{@const retry = t.retry}
			<div class="toast error">
				<button class="dismiss" aria-label="Dismiss" onclick={() => app.dismissToast(t.id)}></button>
				<span class="warning"><Icon name="warning" size={16} heavy /></span>
				<span class="text">{t.text}</span>
				<span class="divider"></span>
				<button
					class="retry"
					onclick={() => {
						app.dismissToast(t.id);
						retry();
					}}>Retry</button
				>
			</div>
		{:else}
			<div class="toast"><span class="text">{t.text}</span></div>
		{/if}
	{/each}
	{#if app.busy}
		<div class="toast"><span class="text">{app.busy}</span></div>
	{/if}
</div>

<style>
	.toasts {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 24px;
		display: flex;
		flex-direction: column-reverse;
		align-items: center;
		gap: 8px;
		pointer-events: none;
		z-index: 100;
		--toast: color-mix(in srgb, var(--ink) 95%, transparent);
		--toast-danger: #f07a6e;
	}

	:global(html[data-theme='dark']) .toasts {
		--toast: rgb(242 240 235 / 0.95);
		--toast-danger: #c4372c;
	}

	.toast {
		position: relative;
		display: flex;
		align-items: center;
		height: 40px;
		padding: 0 16px;
		border-radius: 20px;
		background: var(--toast);
		box-shadow: 0 8px 48px rgb(0 0 0 / 0.18);
		color: var(--paper);
		font: 500 13px/16px var(--font-ui);
	}

	.text {
		max-width: 480px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.error {
		padding-left: 14px;
		pointer-events: auto;
	}

	.dismiss {
		position: absolute;
		inset: 0;
		border-radius: inherit;
		cursor: default;
	}

	.warning {
		display: flex;
		margin-right: 10px;
		color: var(--toast-danger);
		pointer-events: none;
	}

	.error .text {
		pointer-events: none;
	}

	.divider {
		width: 1px;
		height: 16px;
		margin: 0 4px 0 10px;
		background: color-mix(in srgb, var(--paper) 55%, transparent);
	}

	.retry {
		position: relative;
		align-self: stretch;
		margin-right: -6px;
		padding: 0 6px;
		color: inherit;
		font-weight: 600;
	}
</style>
