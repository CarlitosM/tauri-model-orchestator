import { invoke } from '@tauri-apps/api/core';

export type OllamaModel = {
	name: string;
	size: number;
	modifiedAt: string;
};

class ModelsStore {
	models = $state<OllamaModel[]>([]);
	selectedModel = $state<string>('');
	isLoading = $state(false);
	errorMessage = $state<string>('');

	haveModels = $derived(this.models.length > 0);
	hasError = $derived(Boolean(this.errorMessage));
	validSelectedModel = $derived(this.models.some(({ name }) => name === this.selectedModel));

	async loadModels() {
		if (this.isLoading) {
			return;
		}

		this.isLoading = true;
		this.errorMessage = '';

		try {
			this.models = await invoke<OllamaModel[]>('list_models');
		} catch (caughtError) {
			this.errorMessage = caughtError instanceof Error ? caughtError.message : String(caughtError);
		} finally {
			this.isLoading = false;
		}
	}
}

export const modelsStore = new ModelsStore();
