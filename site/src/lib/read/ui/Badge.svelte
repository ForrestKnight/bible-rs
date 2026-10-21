<script lang="ts">
	interface Props {
		label: string;
		kind?: 'neutral' | 'accent' | 'solid' | 'success' | 'danger' | 'outline';
	}

	let { label, kind = 'neutral' }: Props = $props();
</script>

<!-- A badge: 20 pt high, radius 10, 11 pt semibold, 7 pt sides; success has a dot. -->
<span class={['badge', kind]}>{label}</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex-shrink: 0;
		min-width: 20px;
		height: 20px;
		padding: 0 7px;
		border-radius: 10px;
		font: 600 11px/16px var(--font-ui);
		white-space: nowrap;
	}

	.neutral {
		background: var(--fill);
		color: var(--ink-2);
	}

	.accent {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.solid {
		background: var(--accent);
		color: var(--on-accent);
	}

	.success {
		background: color-mix(in srgb, var(--success) 14%, transparent);
		color: var(--success);
	}

	.success::before {
		content: '';
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
	}

	.danger {
		background: color-mix(in srgb, var(--danger) 14%, transparent);
		color: var(--danger);
	}

	.outline {
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--ink-2);
	}
</style>
