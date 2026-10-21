<!--
	The app's icon as onboarding.rs draws it (`app_icon`): the design's 64 pt tile, radius 14.4, a
	160° blue gradient with a soft blue drop and a white book; it scales with `size`.
-->
<script lang="ts">
	import Icon from './Icon.svelte';

	let { size = 64 }: { size?: number } = $props();
</script>

<span class="icon" style:--k={size / 64} aria-hidden="true">
	<Icon name="book" size={size / 2} />
</span>

<style>
	.icon {
		display: grid;
		place-items: center;
		flex: none;
		width: calc(64px * var(--k));
		height: calc(64px * var(--k));
		border-radius: calc(14.4px * var(--k));
		background: linear-gradient(160deg, rgb(61 93 186), rgb(36 57 122));
		box-shadow:
			inset 0 0 0 0.5px rgb(255 255 255 / 0.25),
			0 8px 48px rgb(36 57 122 / 0.3);
		color: #fff;
	}
</style>
