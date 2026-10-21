<script lang="ts">
	interface Props {
		checked: boolean;
		name: string;
		label?: string;
		onchange?: () => void;
		/** Only the dot: the row around it carries the label. */
		bare?: boolean;
	}

	let { checked, name, label, onchange, bare = false }: Props = $props();
</script>

<!-- A radio: 16 pt; on, a 5 pt accent ring around a raised centre. -->
<label class={['radio', { bare }]}>
	<input type="radio" {name} {checked} aria-label={bare ? label : undefined} onchange={() => onchange?.()} />
	<span class="dot" aria-hidden="true"></span>
	{#if label && !bare}<span>{label}</span>{/if}
</label>

<style>
	.radio {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		color: var(--ink);
		font: var(--type-ui);
	}

	input {
		position: absolute;
		opacity: 0;
		width: 16px;
		height: 16px;
		margin: 0;
	}

	.dot {
		display: block;
		flex-shrink: 0;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--field);
		box-shadow: inset 0 0 0 1px var(--border);
	}

	.radio:hover .dot {
		box-shadow: inset 0 0 0 1px var(--ink-3);
	}

	input:checked + .dot {
		background: var(--raised);
		box-shadow: inset 0 0 0 5px var(--accent);
	}

	input:focus-visible + .dot {
		outline: var(--focus-ring-width) solid var(--focus);
		outline-offset: 1px;
	}
</style>
