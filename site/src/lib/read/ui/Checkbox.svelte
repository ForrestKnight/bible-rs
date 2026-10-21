<script lang="ts">
	interface Props {
		checked: boolean;
		label: string;
		onchange?: (checked: boolean) => void;
		disabled?: boolean;
	}

	let { checked = $bindable(), label, onchange, disabled = false }: Props = $props();
</script>

<!-- A checkbox: 16 pt, radius 4; on, the accent with a check; the label 8 pt after. -->
<label class={['checkbox', { disabled }]}>
	<input
		type="checkbox"
		bind:checked
		{disabled}
		onchange={() => onchange?.(checked)}
	/>
	<span class="box" aria-hidden="true">
		<svg viewBox="0 0 16 16" width="16" height="16"><path d="M4 8.25 7 11 12 5.25" /></svg>
	</span>
	<span class="label">{label}</span>
</label>

<style>
	.checkbox {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		color: var(--ink);
		font: var(--type-ui);
	}

	.disabled {
		opacity: var(--disabled-opacity);
	}

	input {
		position: absolute;
		opacity: 0;
		width: 16px;
		height: 16px;
		margin: 0;
	}

	.box {
		display: block;
		flex-shrink: 0;
		width: 16px;
		height: 16px;
		border-radius: 4px;
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
	}

	.checkbox:hover .box {
		box-shadow: inset 0 0 0 1px var(--ink-3);
	}

	input:checked + .box {
		background: var(--accent);
		box-shadow: none;
	}

	input:focus-visible + .box {
		outline: var(--focus-ring-width) solid var(--focus);
		outline-offset: 1px;
	}

	svg {
		display: block;
		fill: none;
		stroke: var(--on-accent);
		stroke-width: 1.75;
		stroke-linecap: round;
		stroke-linejoin: round;
		opacity: 0;
	}

	input:checked + .box svg {
		opacity: 1;
	}
</style>
