<!--
	The toolbar's translation menu (app.rs `translation_menu`): the translations with ⌘1…⌘9 (each a
	link to this chapter in it), then the NLT, read live (../nlt.ts), the others to read side by side,
	and Manage translations….
-->
<script lang="ts">
	import { getApp } from '../app.svelte';
	import { capsText, fixedCaps } from '../keyboard';
	import { Badge, Menu, MenuCheck, MenuHeader, MenuItem, MenuSeparator } from '../ui';

	interface Props {
		anchor: HTMLElement;
		onclose: () => void;
	}

	let { anchor, onclose }: Props = $props();

	const app = getApp();
	const s = $derived(app.settings);

	function act(a: Parameters<typeof app.dispatch>[0]) {
		app.dispatch(a);
		onclose();
	}
</script>

<Menu {anchor} {onclose} label="Translation" width={300} quiet>
	{#each app.translations as t, i (t.id)}
		<MenuItem
			label="{t.abbrev}  {t.name}"
			shortcut={i < 9 ? capsText(fixedCaps(['⌘', String(i + 1)])) : undefined}
			selected={t.id === s.translation}
			href={app.chapterHref(app.here.chapter, t.id)}
			onclick={() => act({ type: 'setTranslation', id: t.id })}
		/>
	{/each}
	{#each app.live as t (t.id)}
		<MenuItem
			label="{t.abbrev}  {t.name}"
			selected={t.id === s.translation}
			href={app.chapterHref(app.here.chapter, t.id)}
			onclick={() => act({ type: 'setTranslation', id: t.id })}
		>
			{#snippet trailing()}<Badge label="Live" />{/snippet}
		</MenuItem>
	{/each}
	<MenuSeparator />
	<MenuHeader label="Parallel view" />
	{#each app.translations.filter((t) => t.id !== s.translation) as t (t.id)}
		<MenuCheck
			label="{t.abbrev} — side by side"
			checked={s.parallelOn && s.parallel.includes(t.id)}
			onchange={(on) => app.dispatch(on ? { type: 'addParallel', id: t.id } : { type: 'removeParallel', id: t.id })}
		/>
	{/each}
	<MenuSeparator />
	<MenuItem label="Manage translations…" onclick={() => act({ type: 'openSettings', tab: 'translations' })} />
</Menu>
