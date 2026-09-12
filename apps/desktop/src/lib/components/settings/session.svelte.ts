import { invoke } from "@tauri-apps/api/core";
import type {
	Channel,
	ApiConfiguration,
	CommandResponse,
	CodexResets,
	ConfigurationStatus,
	NtfyConfig,
	NotionCalendar,
	R2Configuration,
	UgosConfiguration,
} from "../../consumer";

export function createSettingsSession(effects: {
	resetChannel: (channel: Channel) => void;
	initializeConsumers: () => Promise<void>;
	refreshDashboard: () => Promise<void>;
}) {
	let configuration = $state<ConfigurationStatus | null>(null);
	let configurationError = $state<string | null>(null);
	let configurationRequest = 0;
	let spotifyRevision = $state(0);
	async function loadConfiguration() {
		const version = ++configurationRequest;
		configurationError = null;
		const response = await invoke<CommandResponse<ConfigurationStatus>>("read_configuration");
		if (version !== configurationRequest) return;
		if (response.status === "failed") {
			configurationError = response.message;
			return;
		}
		configuration = response.data;
	}

	async function saveUgosConfiguration(input: UgosConfiguration): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_ugos_configuration", {
			username: input.username,
			password: input.password,
		});
		if (response.status === "ready") {
			await loadConfiguration();
			await effects.refreshDashboard();
		}
		return response;
	}

	async function saveR2Configuration(input: R2Configuration): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_r2_configuration", {
			accessKeyId: input.accessKeyId,
			secretAccessKey: input.secretAccessKey,
		});
		if (response.status === "ready") {
			await loadConfiguration();
			await effects.initializeConsumers();
		}
		return response;
	}

	async function saveApiConfiguration(input: ApiConfiguration): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_api_configuration", {
			service: input.service,
			apiKey: input.apiKey,
		});
		if (response.status === "ready") {
			effects.resetChannel(input.service);
			await loadConfiguration();
			await effects.initializeConsumers();
		}
		return response;
	}

	async function connectSpotify(clientId: string): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("connect_spotify", { clientId });
		if (response.status === "ready") {
			spotifyRevision += 1;
			await loadConfiguration();
		}
		return response;
	}

	async function saveNotionCalendar(
		configuration: NotionCalendar,
	): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_notion_calendar", {
			configuration,
		});
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function saveCodexResets(configuration: CodexResets): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_codex_resets", {
			configuration,
		});
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function saveNtfy(configuration: NtfyConfig): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_ntfy_configuration", {
			configuration,
		});
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function saveAppLock(password: string): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_app_lock", { password });
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function removeAppLock(): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("remove_app_lock");
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	return {
		get configuration() {
			return configuration;
		},
		get spotifyRevision() {
			return spotifyRevision;
		},
		get error() {
			return configurationError;
		},
		set error(value: string | null) {
			configurationError = value;
		},
		loadConfiguration,
		saveUgosConfiguration,
		saveR2Configuration,
		saveApiConfiguration,
		connectSpotify,
		saveNtfy,
		saveNotionCalendar,
		saveCodexResets,
		saveAppLock,
		removeAppLock,
	};
}
