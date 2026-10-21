<script lang="ts" generics="T extends string | number">
	import Icon from './Icon.svelte';
	import type { IconName } from './icons';

	interface Props {
		options: { value: T; label: string }[];
		value: T;
		onchange?: (value: T) => void;
		label: string;
		icon?: IconName;
		width?: number | string;
		id?: string;
	}

	let { options, value = $bindable(), onchange, label, icon, width, id }: Props = $props();
</script>

<!-- A pop-up button: the field colour, a hairline and a soft drop, an optional glyph, the value and
     up-down chevrons. The menu itself is the browser's. -->
<div class="select" style:width={typeof width === 'number' ? `${width}px` : width}>
	{#if icon}<Icon name={icon} size={16} heavy class="lead" />{/if}
	<span class="value">{options.find((o) => o.value === value)?.label ?? ''}</span>
	<Icon name="chevrons-up-down" size={16} heavy class="chevrons" />
	<select
		{id}
		aria-label={label}
		value={String(value)}
		onchange={(e) => {
			const o = options[e.currentTarget.selectedIndex];
			if (!o) return;
			value = o.value;
			onchange?.(o.value);
		}}
	>
		{#each options as o (o.value)}
			<option value={String(o.value)}>{o.label}</option>
		{/each}
	</select>
</div>

<style>
	.select {
		position: relative;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 0 8px 0 12px;
		border-radius: 8px;
		background: var(--field);
		box-shadow:
			0 0 0 0.5px var(--border),
			0 1px 2px rgb(0 0 0 / 0.06);
		color: var(--ink);
		font: var(--type-ui);
	}

	.select:hover {
		background: var(--raised);
	}

	.select:focus-within {
		box-shadow:
			0 0 0 1px var(--accent),
			0 0 0 4px var(--focus);
	}

	.select :global(.lead),
	.select :global(.chevrons) {
		color: var(--ink-2);
	}

	.value {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	select {
		position: absolute;
		inset: 0;
		width: 100%;
		opacity: 0;
		font: var(--type-ui);
	}
</style>
