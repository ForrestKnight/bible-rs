<script lang="ts">
	import '#lib/read/read.css';
	import Shell from '#lib/read/Shell.svelte';

	let { data, children } = $props();
</script>

<Shell {data} />
{@render children()}
