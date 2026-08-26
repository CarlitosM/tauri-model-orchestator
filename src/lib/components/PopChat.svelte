<script lang="ts">
	import { popChat } from '$lib/composables/popChat.svelte';
	import { modelsStore } from '$lib/composables/models.svelte';
</script>

<form
	onsubmit={async (event) => {
		event.preventDefault();
		void await popChat.sendChat();
	}}
>
	<label for="chat-prompt">Prompt</label>
	<textarea
		id="chat-prompt"
		rows="4"
		bind:value={popChat.chatPrompt}
		disabled={!modelsStore.selectedModel || popChat.isChatPending}
		placeholder={modelsStore.selectedModel ? 'Type a prompt…' : 'Select a model first'}
	></textarea>
	<button type="submit" disabled={!popChat.chatPrompt.trim() || !modelsStore.selectedModel || popChat.isChatPending}>
		{popChat.isChatPending ? 'Waiting for response…' : 'Send'}
	</button>
</form>

{#if popChat.chatError}
	<p role="alert">Error: {popChat.chatError}</p>
{/if}

{#if popChat.chatResponse}
	<section aria-live="polite">
		<h3>{popChat.chatResponse.model}</h3>
		<p>{popChat.chatResponse.content}</p>
	</section>
{/if}
