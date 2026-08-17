<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { emit } from '@tauri-apps/api/event';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { onMount } from 'svelte';

	type EchoRequest = {
		requestId: string;
		message: string;
	};

	type EchoResponse = EchoRequest & {
		receivedAt: number;
	};

	type OllamaModel = {
		name: string;
		size: number;
		modifiedAt: string;
	};

	type NonStreamChatResponse = {
		model: string;
		content: string;
	};

	type StreamingChatRequest = {
		requestId: string;
		model: string;
		message: string;
	};

	type StreamingChatResponse = {
		requestId: string;
		model: string;
		content: string;
		done: boolean;
		error?: string;
	};

	let message = $state('');
	let response = $state<EchoResponse>();
	let listenerReady = $state(false);
	let isPending = $state(false);
	let error = $state<string>();
	let requestNumber = 0;
	let models = $state<OllamaModel[]>([]);
	let modelsError = $state<string>();
	let isLoadingModels = $state(false);
	let selectedModel = $state<string>();
	let chatPrompt = $state('');
	let chatResponse = $state<NonStreamChatResponse>();
	let isChatPending = $state(false);
	let chatError = $state<string>();

	let streamPrompt = $state('');
	let streamContent = $state('');
	let streamModel = $state('');
	let isStreaming = $state(false);
	let streamError = $state<string>();
	let streamListenerReady = $state(false);
	let currentStreamRequestId = $state<string>();

	$effect(() => {
		let disposed = false;
		const unlistenPromise = getCurrentWebviewWindow().listen<EchoResponse>(
			'backend-echo-response',
			(event) => {
				response = event.payload;
				isPending = false;
			}
		);

		void unlistenPromise.then((unlisten) => {
			if (disposed) {
				unlisten();
				return;
			}

			listenerReady = true;
		});

		return () => {
			disposed = true;
			listenerReady = false;
			void unlistenPromise.then((unlisten) => unlisten());
		};
	});

	$effect(() => {
		let disposed = false;
		const unlistenPromise = getCurrentWebviewWindow().listen<StreamingChatResponse>(
			'streaming-chat-response',
			(event) => {
				const payload = event.payload;

				// Ignore stale events from a previous request
				if (payload.requestId !== currentStreamRequestId) return;

				if (payload.error) {
					streamError = payload.error;
					isStreaming = false;
					currentStreamRequestId = undefined;
					return;
				}

				streamContent += payload.content;
				streamModel = payload.model;

				if (payload.done) {
					isStreaming = false;
					currentStreamRequestId = undefined;
				}
			}
		);

		void unlistenPromise.then((unlisten) => {
			if (disposed) {
				unlisten();
				return;
			}

			streamListenerReady = true;
		});

		return () => {
			disposed = true;
			streamListenerReady = false;
			void unlistenPromise.then((unlisten) => unlisten());
		};
	});


	async function sendEcho() {
		const trimmedMessage = message.trim();

		if (!trimmedMessage || !listenerReady || isPending) {
			return;
		}

		const request: EchoRequest = {
			requestId: `${Date.now()}-${++requestNumber}`,
			message: trimmedMessage
		};

		isPending = true;
		error = undefined;
		response = undefined;

		try {
			await emit('frontend-echo-request', request);
		} catch (caughtError) {
			error = caughtError instanceof Error ? caughtError.message : String(caughtError);
			isPending = false;
		}
	}

	async function sendChat() {
		const trimmedPrompt = chatPrompt.trim();

		if (!trimmedPrompt || !selectedModel || isChatPending) {
			return;
		}

		isChatPending = true;
		chatError = undefined;
		chatResponse = undefined;

		try {
			chatResponse = await invoke<NonStreamChatResponse>('non_stream_chat', {
				model: selectedModel,
				message: trimmedPrompt
			});
		} catch (caughtError) {
			chatError = caughtError instanceof Error ? caughtError.message : String(caughtError);
		} finally {
			isChatPending = false;
		}
	}

	async function sendStreamChat() {
		const trimmedPrompt = streamPrompt.trim();

		if (!trimmedPrompt || !selectedModel || isStreaming || !streamListenerReady) {
			return;
		}

		const requestId = `${Date.now()}-${++requestNumber}`;
		currentStreamRequestId = requestId;
		isStreaming = true;
		streamError = undefined;
		streamContent = '';
		streamModel = '';

		const request: StreamingChatRequest = {
			requestId,
			model: selectedModel,
			message: trimmedPrompt
		};

		try {
			await emit('streaming-chat', request);
		} catch (caughtError) {
			streamError = caughtError instanceof Error ? caughtError.message : String(caughtError);
			isStreaming = false;
			currentStreamRequestId = undefined;
		}
	}


	async function loadModels() {
		if (isLoadingModels) {
			return;
		}

		isLoadingModels = true;
		modelsError = undefined;

		try {
			models = await invoke<OllamaModel[]>('list_ollama_models');
		} catch (caughtError) {
			modelsError = caughtError instanceof Error ? caughtError.message : String(caughtError);
		} finally {
			isLoadingModels = false;
		}
	}

	function formatSize(bytes: number) {
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let value = bytes;
		let unitIndex = 0;

		while (value >= 1024 && unitIndex < units.length - 1) {
			value /= 1024;
			unitIndex += 1;
		}

		return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })} ${units[unitIndex]}`;
	}

onMount(async () => {
	void await loadModels();
});
</script>

<h1>Local Models Orchestrators</h1>
<p><abbr title="Work In Progress">WIP</abbr></p>

<form
	onsubmit={(event) => {
		event.preventDefault();
		void sendEcho();
	}}
>
	<label for="echo-message">Message for Rust</label>
	<input id="echo-message" bind:value={message} disabled={!listenerReady || isPending} />
	<button type="submit" disabled={!message.trim() || !listenerReady || isPending}>
		{isPending ? 'Waiting for Rust…' : 'Send echo'}
	</button>
</form>

<p aria-live="polite">
	{listenerReady ? 'Event bridge connected.' : 'Connecting event bridge…'}
</p>

{#if response}
	<section aria-live="polite">
		<h2>Rust response</h2>
		<p>{response.message}</p>
		<p>Request: {response.requestId}</p>
		<p>Received: {new Date(response.receivedAt).toLocaleString()}</p>
	</section>
{/if}

{#if error}
	<p role="alert">Unable to send message: {error}</p>
{/if}

<section aria-labelledby="chat-heading">
	<h2 id="chat-heading">Chat</h2>

	<form
		onsubmit={(event) => {
			event.preventDefault();
			void sendChat();
		}}
	>
		<label for="chat-prompt">Prompt</label>
		<textarea
			id="chat-prompt"
			rows="4"
			bind:value={chatPrompt}
			disabled={!selectedModel || isChatPending}
			placeholder={selectedModel ? 'Type a prompt…' : 'Select a model first'}
		></textarea>
		<button type="submit" disabled={!chatPrompt.trim() || !selectedModel || isChatPending}>
			{isChatPending ? 'Waiting for response…' : 'Send'}
		</button>
	</form>

	{#if chatError}
		<p role="alert">Error: {chatError}</p>
	{/if}

	{#if chatResponse}
		<section aria-live="polite">
			<h3>{chatResponse.model}</h3>
			<p>{chatResponse.content}</p>
		</section>
	{/if}
</section>

<section aria-labelledby="stream-chat-heading">
	<h2 id="stream-chat-heading">Streaming Chat</h2>

	<p aria-live="polite">
		{streamListenerReady ? 'Stream bridge connected.' : 'Connecting stream bridge…'}
	</p>

	<form
		onsubmit={(event) => {
			event.preventDefault();
			void sendStreamChat();
		}}
	>
		<label for="stream-prompt">Prompt</label>
		<textarea
			id="stream-prompt"
			rows="4"
			bind:value={streamPrompt}
			disabled={!selectedModel || isStreaming}
			placeholder={selectedModel ? 'Type a streaming prompt…' : 'Select a model first'}
		></textarea>
		<button
			type="submit"
			disabled={!streamPrompt.trim() || !selectedModel || isStreaming || !streamListenerReady}
		>
			{isStreaming ? 'Streaming…' : 'Send (stream)'}
		</button>
	</form>

	{#if streamError}
		<p role="alert">Stream error: {streamError}</p>
	{/if}

	{#if streamContent || isStreaming}
		<section aria-live="polite">
			{#if streamModel}
				<h3>{streamModel}</h3>
			{/if}
			<p style="white-space: pre-wrap">{streamContent}{isStreaming ? '▍' : ''}</p>
		</section>
	{/if}
</section>


<section aria-labelledby="ollama-models-heading">
	<h2 id="ollama-models-heading">Installed Ollama models</h2>

	{#if selectedModel}
		<p>Selected model: <strong>{selectedModel}</strong></p>
	{:else}
		<p>No model selected.</p>
	{/if}

	<button type="button" onclick={() => void loadModels()} disabled={isLoadingModels}>
		{isLoadingModels ? 'Loading models…' : 'Refresh models'}
	</button>

	{#if modelsError}
		<p role="alert">Unable to load Ollama models: {modelsError}</p>
	{:else if isLoadingModels}
		<p aria-live="polite">Loading installed Ollama models…</p>
	{:else if models.length === 0}
		<p>No Ollama models are installed.</p>
	{:else}
		<table>
			<thead>
				<tr>
					<th scope="col">Select</th>
					<th scope="col">Name</th>
					<th scope="col">Size</th>
					<th scope="col">Modified</th>
				</tr>
			</thead>
			<tbody>
				{#each models as model (model.name)}
					<tr>
						<td>
							<input
								type="radio"
								name="model-select"
								id="model-{model.name}"
								value={model.name}
								bind:group={selectedModel}
							/>
						</td>
						<td><label for="model-{model.name}">{model.name}</label></td>
						<td>{formatSize(model.size)}</td>
						<td>{new Date(model.modifiedAt).toLocaleString()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</section>
