<script lang="ts">
	interface Props {
		label: string;
		count?: string | number;
		/** A colour dot before the label (highlight colours). */
		dot?: string;
		selected: boolean;
		onclick: () => void;
	}

	let { label, count, dot, selected, onclick }: Props = $props();
</script>

<!-- A pill tab: 28 pt, radius 14, 13 pt medium; selected, accent on accent-soft. -->
<button type="button" class={['pill', { selected }]} role="tab" aria-selected={selected} {onclick}>
	{#if dot}<span class="dot" style:background={dot}></span>{/if}
	<span>{label}</span>
	{#if count !== undefined}<span class="count">{count}</span>{/if}
</button>

<style>
	.pill {
		display: inline-flex;
		align-items: center;
		flex-shrink: 0;
		height: 28px;
		padding: 0 12px;
		border-radius: 14px;
		color: var(--ink-2);
		font: 500 13px/20px var(--font-ui);
		white-space: nowrap;
		transition: background-color var(--motion-fast);
	}

	.pill:hover:not(.selected) {
		background: var(--fill);
	}

	.selected {
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 600;
	}

	.dot {
		width: 10px;
		height: 10px;
		margin-right: 6px;
		border-radius: 50%;
	}

	.count {
		margin-left: 6px;
		color: var(--ink-3);
		font: 500 12px/16px var(--font-ui);
	}

	.selected .count {
		color: var(--accent);
	}
</style>
