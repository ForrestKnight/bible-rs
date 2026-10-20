<script lang="ts">
	import type { Place } from '#lib/demo/script.js';
	import Icon, { type IconName } from './Icon.svelte';
	import Keys from './Keys.svelte';

	type Command = { icon: IconName; label: string; keys: string[] };

	/** The ⌘K palette: a reference field, the places it matches, and commands for the query. */
	let {
		query,
		caret,
		places,
		commands,
		pressed = false,
		hints = true
	}: { query: string; caret: boolean; places: Place[]; commands: Command[]; pressed?: boolean; hints?: boolean } =
		$props();
</script>

<div class="palette">
	<div class="input">
		<Icon name="search" size={16} />
		<span class="query">{query}</span>
		<span class="caret" style:opacity={caret ? 1 : 0}></span>
		{#if !query}<span class="placeholder">Go to a reference, search, or run a command</span>{/if}
		<span class="spacer"></span>
		<Keys keys={['esc']} />
	</div>
	{#if places.length || commands.length}
		<div class="list">
			{#if places.length}
				<div class="section">Go to</div>
				{#each places as place, i (place.ref.map((p) => p[0]).join(''))}
					<div class={['row', { selected: i === 0, pressed: pressed && i === 0 }]}>
						<span class="badge"><Icon name={place.kind} size={13} /></span>
						<span class="body">
							<span class="title"
								>{#each place.ref as [text, matched], j (j)}<span class={{ matched }}>{text}</span>{/each}</span
							>
							<span class={['detail', { serif: place.kind === 'verse' }]}>{place.detail}</span>
						</span>
						<span class="tr">KJV</span>
						{#if i === 0}<Keys keys={['↵']} />{/if}
					</div>
				{/each}
			{/if}
			{#if commands.length}
				<div class="section">Commands</div>
				{#each commands as command (command.label)}
					<div class="row">
						<span class="badge"><Icon name={command.icon} size={13} /></span>
						<span class="body"><span class="title">{command.label}</span></span>
						<Keys keys={command.keys} />
					</div>
				{/each}
			{/if}
		</div>
	{/if}
	{#if hints}
		<div class="hints">
			<span><Keys keys={['↑↓']} size={16} />Move</span>
			<span><Keys keys={['↵']} size={16} />Open</span>
			<span><Keys keys={['⌘↵']} size={16} />Open in parallel</span>
			<span class="tip">Type a reference, book or command</span>
		</div>
	{/if}
</div>

<style>
	.palette {
		container-type: inline-size;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: 13px;
		background: var(--popover);
		box-shadow: var(--popover-edge);
		color: var(--ink);
		font-family: var(--sans);
	}

	.input {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 46px;
		padding: 0 12px 0 14px;
		color: var(--ink-2);
	}

	.query {
		color: var(--ink);
		font-size: 15px;
		white-space: pre;
	}

	.caret {
		width: 1.5px;
		height: 18px;
		margin-left: -9px;
		background: var(--accent);
	}

	.placeholder {
		margin-left: -9px;
		color: var(--ink-3);
		font-size: 15px;
	}

	.spacer {
		flex: 1;
	}

	.list {
		padding: 0 6px 6px;
		border-top: 1px solid var(--separator);
	}

	.section {
		padding: 10px 8px 5px;
		color: var(--ink-2);
		font-size: 10px;
		font-weight: 600;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 34px;
		padding: 5px 8px;
		border-radius: 8px;
		transition: background-color 0.15s;
	}

	.row.selected {
		background: var(--accent-soft);
	}

	.row.pressed {
		background: color-mix(in srgb, var(--accent) 34%, var(--accent-soft));
	}

	.badge {
		display: grid;
		flex-shrink: 0;
		place-items: center;
		width: 24px;
		height: 24px;
		border-radius: 6px;
		background: var(--chip);
		color: var(--ink-2);
	}

	.body {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}

	.title {
		font-size: 12.5px;
		font-weight: 600;
		line-height: 16px;
	}

	.matched {
		color: var(--accent);
	}

	.detail {
		overflow: hidden;
		color: var(--ink-2);
		font-size: 11px;
		line-height: 14px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.detail.serif {
		font-family: var(--serif);
	}

	.tr {
		color: var(--ink-2);
		font-size: 10px;
		font-weight: 500;
	}

	.hints {
		display: flex;
		gap: 14px;
		height: 30px;
		align-items: center;
		padding: 0 12px;
		border-top: 1px solid var(--separator);
		background: color-mix(in srgb, var(--ink) 3%, transparent);
		color: var(--ink-3);
		font-size: 10px;
	}

	.hints span {
		display: flex;
		align-items: center;
		gap: 5px;
	}

	.tip {
		margin-left: auto;
	}

	@container (max-width: 440px) {
		.hints .tip {
			display: none;
		}
	}
</style>
