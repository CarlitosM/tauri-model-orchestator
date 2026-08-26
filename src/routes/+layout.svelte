<script lang="ts">
	import { onMount } from 'svelte';
	import { modelsStore } from '$lib/composables/models.svelte';

	const { children } = $props();

	onMount(() => {
		void modelsStore.loadModels();
	});
</script>

{@render children()}
