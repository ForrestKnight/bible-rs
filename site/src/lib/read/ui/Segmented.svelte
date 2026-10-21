<script lang="ts" generics="T extends string | number">
	interface Props {
		options: { value: T; label: string }[];
		value: T;
		onchange?: (value: T) => void;
		/** Full width with equal segments; `fit` sizes segments to their labels (13 pt sides),
		 * `tight` to 10 pt sides with 2 pt between. */
		layout?: 'fill' | 'fit' | 'tight';
		label?: string;
		class?: string;
	}

	let { options, value = $bindable(), onchange, layout = 'fill', label, class: cls }: Props = $props();

	function pick(v: T) {
		if (v === value) return;
		value = v;
		onchange?.(v);
	}

	function keydown(e: KeyboardEvent) {
		const at = options.findIndex((o) => o.value === value);
		const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
		if (!d) return;
		e.preventDefault();
		const next = options[(at + d + options.length) % options.length];
		pick(next.value);
		(e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-i="${options.indexOf(next)}"]`)?.focus();
	}
</script>

<div class={['track', layout, cls]} role="radiogroup" aria-label={label} tabindex="-1" onkeydown={keydown}>
	{#each options as o, i (o.value)}
		<button
			type="button"
			class={['segment', { on: o.value === value }]}
			role="radio"
			aria-checked={o.value === value}
			tabindex={o.value === value ? 0 : -1}
			data-i={i}
			onclick={() => pick(o.value)}
		>
			{o.label}
		</button>
	{/each}
</div>

<style>
	.track {
		display: flex;
		height: 32px;
		padding: 2px;
		border-radius: 16px;
		background: var(--fill);
		outline: none;
	}

	.fill {
		width: 100%;
	}

	.fill .segment {
		flex: 1 1 0;
	}

	.fit .segment {
		padding: 0 13px;
	}

	.tight {
		gap: 2px;
	}

	.tight .segment {
		padding: 0 10px;
	}

	.segment {
		height: 28px;
		border-radius: 14px;
		color: var(--ink-2);
		font: 500 13px/20px var(--font-ui);
		white-space: nowrap;
		transition: background-color var(--motion-fast);
	}

	.segment:hover:not(.on) {
		background: var(--fill);
	}

	.segment.on {
		background: var(--raised);
		box-shadow: var(--shadow-knob);
		color: var(--ink);
		font-weight: 600;
	}
</style>
