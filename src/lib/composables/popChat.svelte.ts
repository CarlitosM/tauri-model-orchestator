import { invoke } from '@tauri-apps/api/core';
import { modelsStore } from '$lib/composables/models.svelte';

export type NonStreamChatResponse = {
	model: string;
	content: string;
};

class PopChat {
  chatPrompt = $state('');
  chatResponse = $state<NonStreamChatResponse>();
  isChatPending = $state(false);
  chatError = $state<string>();

  async sendChat() {
		const trimmedPrompt = this.chatPrompt.trim();

		if (!trimmedPrompt || !modelsStore.selectedModel || this.isChatPending) {
			return;
		}

		this.isChatPending = true;
		this.chatError = undefined;
		this.chatResponse = undefined;

		try {
			this.chatResponse = await invoke<NonStreamChatResponse>('non_stream_chat', {
				model: modelsStore.selectedModel,
				message: trimmedPrompt
			});
		} catch (caughtError) {
			this.chatError = caughtError instanceof Error ? caughtError.message : String(caughtError);
		} finally {
			this.isChatPending = false;
		}
	}
}

export const popChat = new PopChat();
