<script lang="ts">
	interface Props {
		value: number;
		min: number;
		max: number;
		step: number;
		/** Tick marks under the track (0 for none). */
		ticks?: number;
		label: string;
		/** How the value reads aloud ("20 points"). */
		valueText?: string;
		oninput?: (value: number) => void;
		id?: string;
	}

	let { value = $bindable(), min, max, step, ticks = 0, label, valueText, oninput, id }: Props = $props();

	const t = $derived(Math.min(1, Math.max(0, (value - min) / (max - min))));
</script>

<!-- A slider: a 4 pt track with the accent up to the knob, a 20 pt white knob, optional ticks. -->
<div class="slider" style:--t={t}>
	<input
		{id}
		type="range"
		{min}
		{max}
		{step}
		bind:value
		aria-label={label}
		aria-valuetext={valueText}
		oninput={() => oninput?.(value)}
	/>
	{#if ticks > 1}
		<div class="ticks" aria-hidden="true">
			{#each { length: ticks }, k (k)}
				<span style:left="calc(10px + (100% - 20px) * {k / (ticks - 1)})"></span>
			{/each}
		</div>
	{/if}
</div>

<style>
	.slider {
		position: relative;
		height: 24px;
	}

	input {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 24px;
		margin: 0;
		background: none;
		appearance: none;
		cursor: default;
	}

	input::-webkit-slider-runnable-track {
		height: 4px;
		margin: 0 10px;
		border-radius: 2px;
		background: linear-gradient(var(--accent), var(--accent)) 0 0 / calc(var(--t) * 100%) 100% no-repeat,
			var(--fill-strong);
	}

	input::-moz-range-track {
		height: 4px;
		border-radius: 2px;
		background: var(--fill-strong);
	}

	input::-moz-range-progress {
		height: 4px;
		border-radius: 2px;
		background: var(--accent);
	}

	input::-webkit-slider-thumb {
		width: 20px;
		height: 20px;
		margin-top: -8px;
		border-radius: 50%;
		background: #fff;
		box-shadow: var(--shadow-knob);
		appearance: none;
		transition: box-shadow var(--motion-fast);
	}

	input::-moz-range-thumb {
		width: 20px;
		height: 20px;
		border: 0;
		border-radius: 50%;
		background: #fff;
		box-shadow: var(--shadow-knob);
	}

	input:hover::-webkit-slider-thumb {
		box-shadow:
			var(--shadow-knob),
			0 0 0 4px var(--fill-strong);
	}

	input:active::-webkit-slider-thumb,
	input:focus-visible::-webkit-slider-thumb {
		box-shadow:
			var(--shadow-knob),
			0 0 0 3px var(--focus);
	}

	input:focus-visible {
		outline: none;
	}

	.ticks span {
		position: absolute;
		top: 17px;
		width: 2px;
		height: 4px;
		margin-left: -1px;
		border-radius: 1px;
		background: var(--ink-3);
	}
</style>
