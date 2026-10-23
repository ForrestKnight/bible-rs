<!--
	The app's icon as `app_icon` in onboarding.rs draws it. The logo fills the design's 64 pt tile
	over a soft shadow and scales with `size`.
-->
<script lang="ts">
	let { size = 64 }: { size?: number } = $props();
</script>

<img class="icon" src="/logo.png" width={size} height={size} style:--k={size / 64} alt="" />

<style>
	.icon {
		display: block;
		flex: none;
		border-radius: calc(14.4px * var(--k));
		box-shadow: 0 8px 48px rgb(62 21 23 / 0.3);
	}
</style>
