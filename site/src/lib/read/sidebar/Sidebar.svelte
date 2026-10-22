<!--
	The sidebar (crates/app/src/sidebar.rs): a floating source list (study, history, plans, the
	library and the books), or the search's filters and results while there's a query. Its header
	holds the sidebar toggle; typing over the list filters the books.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { getApp } from '../app.svelte';
	import { bookOf } from '../books';
	import { shortcutText } from '../keyboard';
	import { ToolbarButton } from '../ui';
	import SearchPanel from './SearchPanel.svelte';
	import SourceList from './SourceList.svelte';
	import { SourceListState } from './state.svelte';

	const app = getApp();
	const st = new SourceListState(bookOf(app.books, app.here.chapter));

	let hovered = false;
	let mounted = $state(false);

	const attached = $derived(app.settings.sidebarAttached);
	const searching = $derived(app.search.searching);
	const label = $derived(app.settings.sidebarOpen ? 'Hide sidebar' : 'Show sidebar');
	const keys = $derived(mounted ? shortcutText('sidebar', app.settings.shortcuts) : '');

	onMount(() => {
		mounted = true;
		// Typing over the list (with nothing else taking keys) starts filtering the books, ahead of
		// the reader's single-key shortcuts.
		const keydown = (e: KeyboardEvent) => {
			if (!hovered || searching || st.filtering || !app.settings.sidebarOpen || app.focus) return;
			if (e.ctrlKey || e.metaKey || e.altKey || e.isComposing) return;
			if (e.key.length !== 1 || !/[\p{L}\p{N}]/u.test(e.key)) return;
			const t = document.activeElement;
			if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || (t as HTMLElement)?.isContentEditable)
				return;
			if (document.querySelector('[role="dialog"], [role="menu"]')) return;
			e.preventDefault();
			e.stopPropagation();
			st.startFilter(e.key);
		};
		window.addEventListener('keydown', keydown, { capture: true });
		return () => window.removeEventListener('keydown', keydown, { capture: true });
	});
</script>

<div
	class={['panel', { attached }]}
	role="navigation"
	aria-label="Sidebar"
	onpointerenter={() => (hovered = true)}
	onpointerleave={() => (hovered = false)}
>
	<div class="header">
		<div class="toggle">
			<ToolbarButton
				icon="sidebar"
				width={32}
				label={keys ? `${label} (${keys})` : label}
				onclick={() => app.dispatch({ type: 'toggleSidebar' })}
			/>
		</div>
	</div>
	<div class="content">
		{#if searching}
			<SearchPanel />
		{:else}
			<SourceList {st} />
		{/if}
	</div>
</div>

<style>
	/* Floating: 8 pt in from the window's edges, radius 16, the panel shadow. Attached: the
	   window's own edge, with a hairline beside the content. */
	.panel {
		position: absolute;
		top: 8px;
		bottom: 8px;
		left: 8px;
		right: 0;
		display: flex;
		flex-direction: column;
		border-radius: var(--radius-lg);
		background: var(--sidebar);
		box-shadow: var(--shadow-panel);
		color: var(--ink);
	}

	.panel.attached {
		inset: 0;
		border-radius: 0;
		box-shadow: inset -1px 0 0 var(--separator);
	}

	.header {
		position: relative;
		flex: 0 0 36px;
	}

	/* The toggle's centre is 20 pt in from the panel's edge, on the toolbar's centre line. */
	.toggle {
		position: absolute;
		top: 2px;
		right: 4px;
	}

	.attached .toggle {
		top: 10px;
	}

	.content {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		border-radius: 0 0 var(--radius-lg) var(--radius-lg);
	}

	.attached .content {
		border-radius: 0;
	}
</style>
