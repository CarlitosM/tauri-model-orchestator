import { invoke, Channel } from '@tauri-apps/api/core';
import { modelsStore } from '$lib/composables/models.svelte';

// ── Rust type mirrors ──────────────────────────────────────────────

export type OllamaChatMessage = {
	role: string;
	content: string;
};

export type OllamaChatResponse = {
	model: string;
	message: OllamaChatMessage;
	done: boolean;
};

/**
 * Discriminated union matching the Rust `StreamChatEvent` enum.
 *
 * Rust serde attributes `tag = "event"`, `content = "data"`, `rename_all = "camelCase"`
 * produce this adjacently-tagged JSON shape.
 */
export type StreamChatEvent =
	| { event: 'chatResponse'; data: OllamaChatResponse }
	| { event: 'chatFinished'; data: OllamaChatResponse };

type OllamaChatRequest = {
	model: string;
	messages: OllamaChatMessage[];
};

// ── Composable ─────────────────────────────────────────────────────

class StreamChat {
	prompt = $state('');
	content = $state('');
	model = $state('');
	isStreaming = $state(false);
	error = $state<string>();

	async sendStreamChat() {
		const trimmedPrompt = this.prompt.trim();

		if (!trimmedPrompt || !modelsStore.selectedModel || this.isStreaming) {
			return;
		}

		this.isStreaming = true;
		this.error = undefined;
		this.content = '';
		this.model = '';

		const onEvent = new Channel<StreamChatEvent>();

		onEvent.onmessage = (message) => {
			switch (message.event) {
				case 'chatResponse':
					this.content += message.data.message.content;
					this.model = message.data.model;
					break;
				case 'chatFinished':
					this.content += message.data.message.content;
					this.model = message.data.model;
					this.isStreaming = false;
					break;
			}
		};

		const request: OllamaChatRequest = {
			model: modelsStore.selectedModel,
			messages: [{ role: 'user', content: trimmedPrompt }],
		};

		try {
			await invoke('stream_chat', { request, onEvent });
		} catch (caughtError) {
			this.error =
				caughtError instanceof Error
					? caughtError.message
					: String(caughtError);
			this.isStreaming = false;
		}
	}
}

export const streamChat = new StreamChat();
