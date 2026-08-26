<script lang="ts">
  import { streamChat } from "$lib/composables/streamChat.svelte";
  import { modelsStore } from "$lib/composables/models.svelte";
</script>

<form
		onsubmit={async (event) => {
			event.preventDefault();
			await streamChat.sendStreamChat();
		}}
	>
		<label for="stream-prompt">Prompt</label>
		<textarea
			id="stream-prompt"
			rows="4"
			bind:value={streamChat.prompt}
			disabled={!modelsStore.selectedModel || streamChat.isStreaming}
			placeholder={modelsStore.selectedModel ? 'Type a streaming prompt…' : 'Select a model first'}
		></textarea>
		<button
			type="submit"
			disabled={!streamChat.prompt.trim() || !modelsStore.selectedModel || streamChat.isStreaming}
		>
			{streamChat.isStreaming ? 'Streaming…' : 'Send (stream)'}
		</button>
	</form>

	{#if streamChat.error}
		<p role="alert">Stream error: {streamChat.error}</p>
	{/if}

	{#if streamChat.content || streamChat.isStreaming}
		<section aria-live="polite">
			{#if streamChat.model}
				<h3>{streamChat.model}</h3>
			{/if}
			<p style="white-space: pre-wrap">{streamChat.content}{streamChat.isStreaming ? '▍' : ''}</p>
		</section>
	{/if}
