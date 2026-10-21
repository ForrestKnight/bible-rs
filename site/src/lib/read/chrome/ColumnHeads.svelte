<script lang="ts">
	import { getApp } from '../app.svelte';
	import { Icon, Menu, MenuItem, MenuSeparator } from '../ui';

	/** Below the toolbar: the heads 16 pt down, their hairline 50 pt below that. */
	let { top }: { top: number } = $props();

	const app = getApp();
	const cols = $derived(app.columns());

	let open = $state<number | null>(null);
	const buttons: HTMLElement[] = $state([]);

	function pick(i: number, id: string) {
		open = null;
		app.dispatch(i === 0 ? { type: 'setTranslation', id } : { type: 'replaceParallel', index: i - 1, id });
	}
</script>

<!-- Over parallel columns: each translation's abbreviation with a chevron (a menu to change or
     remove it) and its name below, on the columns' own geometry, then a hairline across them. -->
<div class="heads" style:top="{top + 16}px">
	<div class="cols" style:--n={cols.length}>
		{#each cols as id, i (id)}
			{@const t = app.info(id)}
			<div class="head">
				<button
					bind:this={buttons[i]}
					type="button"
					class="pick"
					aria-haspopup="menu"
					aria-expanded={open === i}
					onclick={() => (open = open === i ? null : i)}
				>
					<span class="abbrev">{t?.abbrev ?? id.toUpperCase()}</span>
					<Icon name="chevron-down" size={12} />
				</button>
				<span class="name">{t?.name ?? ''}</span>
			</div>
		{/each}
	</div>
</div>

{#if open !== null && buttons[open]}
	{@const i = open}
	{@const current = cols[i]}
	<Menu anchor={buttons[i]} quiet width={240} onclose={() => (open = null)} label="Column translation">
		{#each app.translations as t (t.id)}
			<MenuItem label="{t.abbrev}  {t.name}" selected={t.id === current} onclick={() => pick(i, t.id)} />
		{/each}
		{#if i > 0}
			<MenuSeparator />
			<MenuItem
				label="Remove column"
				onclick={() => {
					open = null;
					app.dispatch({ type: 'removeParallel', id: current });
				}}
			/>
		{/if}
	</Menu>
{/if}

<style>
	.heads {
		position: absolute;
		left: 0;
		right: 0;
		container-type: inline-size;
		pointer-events: none;
		z-index: 1;
	}

	/* The parallel columns' geometry (Reader.svelte): 8 pt padded rows, columns 32 pt apart, up to
	   20 em each. */
	.cols {
		--col-gap: round(calc(32 * var(--u)), 1px);
		--col-w: min(20em, calc((100cqw - 40 * var(--u) - 16px - var(--col-gap) * (var(--n) - 1)) / var(--n)));
		--total: calc(var(--col-w) * var(--n) + var(--col-gap) * (var(--n) - 1));
		display: flex;
		gap: var(--col-gap);
		width: calc(var(--total) + 16px);
		height: 50px;
		margin: 0 0 0 max(calc(24 * var(--u)), calc((100cqw - var(--total)) / 2 - 8px));
		padding: 0 8px;
		box-shadow: inset 0 -1px 0 var(--separator);
		font-size: var(--reading-size);
	}

	.head {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 2px;
		width: var(--col-w);
		min-width: 0;
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 16px;
		margin: -4px;
		padding: 4px;
		box-sizing: content-box;
		border-radius: 6px;
		color: var(--ink);
		font: var(--type-ui-strong);
		pointer-events: auto;
	}

	.pick :global(svg) {
		color: var(--ink-2);
	}

	.name {
		max-width: 100%;
		overflow: hidden;
		color: var(--ink-3);
		font: 500 11px/16px var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
