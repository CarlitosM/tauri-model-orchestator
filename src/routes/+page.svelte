<script lang="ts">
	import ModelsList from '$lib/components/ModelsList.svelte';
	import { modelsStore } from '$lib/composables/models.svelte';
	import PopChat from '$lib/components/PopChat.svelte';
	import StreamChat from '$lib/components/StreamChat.svelte';

</script>

<h1>Local Models Orchestrators</h1>
<p><abbr title="Work In Progress">WIP</abbr></p>

<section aria-labelledby="chat-heading">
	<h2 id="chat-heading">Chat</h2>

	<PopChat />
</section>

<section aria-labelledby="stream-chat-heading">
	<h2 id="stream-chat-heading">Streaming Chat</h2>

	<StreamChat />
</section>


<section aria-labelledby="ollama-models-heading">
	<h2 id="ollama-models-heading">Installed Ollama models</h2>

	{#if modelsStore.selectedModel}
		<p>Selected model: <strong>{modelsStore.selectedModel}</strong></p>
	{:else}
		<p>No model selected.</p>
	{/if}

	<button type="button" onclick={() => void modelsStore.loadModels()} disabled={modelsStore.isLoading}>
		{modelsStore.isLoading ? 'Loading models…' : 'Refresh models'}
	</button>

	<ModelsList />
</section>
