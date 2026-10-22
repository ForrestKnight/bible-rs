<script lang="ts">
	import { Icon, type IconName } from '../ui';
	import type { Mark } from './marks';
	import Ring from './Ring.svelte';

	interface Props {
		icon?: IconName;
		/** Recent places use tertiary ink rather than the accent. */
		quietIcon?: boolean;
		label: string;
		/** A small label at the end (a translation's abbreviation). */
		trailing?: string;
		/** Extra space before the label (book rows line up with the labels of icon rows). */
		indent?: number;
		count?: string | number | null;
		/** A disclosure chevron, open or closed. */
		disclosure?: boolean;
		/** The closed chevron shows only under the pointer. */
		disclosureOnHover?: boolean;
		selected?: boolean;
		/** A book's highlights and notes, as a small ring in the icon lane. */
		marks?: Mark[];
		title?: string;
		onclick: (e: MouseEvent) => void;
		/** A click on the chevron (when it does something other than the row). */
		onchevron?: () => void;
		el?: HTMLElement;
	}

	let {
		icon,
		quietIcon = false,
		label,
		trailing,
		indent = 0,
		count,
		disclosure,
		disclosureOnHover = false,
		selected = false,
		marks,
		title,
		onclick,
		onchevron,
		el = $bindable()
	}: Props = $props();
</script>

<!-- A source-list row: 32 pt, 8 pt in, a 16 pt icon in the accent, the label 8 pt after it, a
     count and a disclosure chevron at the end. -->
<button
	bind:this={el}
	type="button"
	class={['row', { selected }]}
	aria-current={selected || undefined}
	aria-expanded={disclosure}
	{title}
	{onclick}
>
	{#if indent}
		<span style:width="{indent}px" class="indent">
			{#if marks?.length}<Ring {marks} size={13} gap={1.75} cap={8} />{/if}
		</span>
	{/if}
	{#if icon}<Icon name={icon} size={16} class={quietIcon ? 'lead quiet' : 'lead'} />{/if}
	<span class="label">{label}</span>
	{#if count !== undefined && count !== null}<span class="count">{count}</span>{/if}
	{#if trailing}<span class="trailing">{trailing}</span>{/if}
	{#if disclosure !== undefined}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<span
			class={['chevron', { hover: disclosureOnHover && !disclosure }]}
			onclick={onchevron
				? (e) => {
						e.stopPropagation();
						onchevron();
					}
				: undefined}
		>
			<Icon name={disclosure ? 'chevron-down' : 'disclosure'} size={12} />
		</span>
	{/if}
</button>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		height: 32px;
		padding: 0 8px;
		border-radius: var(--radius-sm);
		color: var(--ink);
		font: var(--type-ui);
		text-align: left;
		transition: background-color 100ms;
	}

	.row:hover {
		background: var(--fill);
	}

	.row:active {
		background: var(--fill-strong);
	}

	.row.selected {
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 600;
	}

	.indent {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		/* A book's ring sits in the 16 pt icon lane. */
		padding-right: 8px;
		margin-right: -8px;
	}

	.row :global(.lead) {
		flex-shrink: 0;
		color: var(--accent);
	}

	.row :global(.lead.quiet) {
		color: var(--ink-3);
	}

	.label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.count {
		color: var(--ink-3);
		font: var(--type-small);
		font-variant-numeric: tabular-nums;
	}

	.selected .count {
		color: var(--accent);
		font-weight: 500;
	}

	.trailing {
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
	}

	.chevron {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		width: 16px;
		height: 16px;
		color: var(--ink-3);
	}

	.selected .chevron {
		color: var(--accent);
	}

	.chevron.hover {
		display: none;
	}

	.row:hover .chevron.hover {
		display: grid;
	}
</style>
