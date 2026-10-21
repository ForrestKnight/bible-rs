<script lang="ts">
	interface Props {
		checked: boolean;
		onchange?: (checked: boolean) => void;
		label: string;
		disabled?: boolean;
		id?: string;
	}

	let { checked = $bindable(), onchange, label, disabled = false, id }: Props = $props();
</script>

<!-- A switch: 40 × 24, the accent when on, a 20 pt white knob (grey when off in the dark). -->
<button
	type="button"
	{id}
	class={['switch', { on: checked }]}
	role="switch"
	aria-checked={checked}
	aria-label={label}
	{disabled}
	onclick={() => {
		checked = !checked;
		onchange?.(checked);
	}}
>
	<span class="knob"></span>
</button>

<style>
	.switch {
		position: relative;
		flex-shrink: 0;
		width: 40px;
		height: 24px;
		border-radius: 12px;
		background: var(--fill-strong);
		transition: background-color var(--motion-fast);
	}

	.switch.on {
		background: var(--accent);
	}

	.switch:hover:not(:disabled) {
		background-image: linear-gradient(rgb(0 0 0 / 0.047), rgb(0 0 0 / 0.047));
	}

	.switch:disabled {
		opacity: var(--disabled-opacity);
	}

	.knob {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: #fff;
		box-shadow: var(--shadow-knob);
		transition:
			transform var(--motion-fast) var(--motion-easing),
			background-color var(--motion-fast);
	}

	:global(html[data-theme='dark']) .switch:not(.on) .knob {
		background: #c9c9ce;
	}

	.on .knob {
		transform: translateX(16px);
	}
</style>
