<script lang="ts">
	import { emit } from '@tauri-apps/api/event';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

	type EchoRequest = {
		requestId: string;
		message: string;
	};

	type EchoResponse = EchoRequest & {
		receivedAt: number;
	};

	let message = $state('');
	let response = $state<EchoResponse>();
	let listenerReady = $state(false);
	let isPending = $state(false);
	let error = $state<string>();
	let requestNumber = 0;

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
