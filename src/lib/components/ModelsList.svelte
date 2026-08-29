<script lang="ts">
	import { modelsStore } from '$lib/composables/models.svelte';
	import { formatSize } from '$lib/utils/formatting';
</script>

{#if modelsStore.errorMessage}
	<p role="alert">Unable to load Ollama models: {modelsStore.errorMessage}</p>
{:else if modelsStore.isLoading}
	<p aria-live="polite">Loading installed Ollama models…</p>
{:else if modelsStore.models.length === 0}
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
			{#each modelsStore.models as model (model.name)}
				<tr>
					<td>
						<input
							type="radio"
							name="model-select"
							id="model-{model.name}"
							value={model.name}
							bind:group={modelsStore.selectedModel}
						/>
					</td>
					<td><label for="model-{model.name}">{model.name}</label></td>
					<td>{formatSize(model.size)}</td>
					<td>{(new Intl.DateTimeFormat("en-US").format(new Date(model.modified_at)))}</td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}
