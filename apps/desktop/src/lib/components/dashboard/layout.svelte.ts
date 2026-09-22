import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type { CommandResponse, ServiceStatusCatalogEntry, WidgetLayout } from "../../consumer";

export function createLayoutSession() {
	let layout = $state<WidgetLayout | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state<string | null>(null);
	let serviceCatalog = $state<ServiceStatusCatalogEntry[]>([]);
	let serviceCatalogError = $state<string | null>(null);
	let revision = 0;
	let disposed = false;
	let islandAvailable = $state(false);

	async function load() {
		if (disposed) return;
		const version = ++revision;
		const response = await invoke<CommandResponse<WidgetLayout>>("read_layout");
		if (version !== revision) return;
		loading = false;
		if (response.status === "ready") {
			layout = response.data;
			error = null;
		} else error = response.message;
	}

	async function save(next: WidgetLayout) {
		if (disposed || saving) return false;
		const version = ++revision;
		saving = true;
		const response = await invoke<CommandResponse<null>>("save_layout", { layout: next });
		if (disposed) return false;
		saving = false;
		if (version !== revision) return response.status === "ready";
		loading = false;
		if (response.status === "failed") {
			error = response.message;
			return false;
		}
		layout = next;
		error = null;
		return true;
	}

	async function reset() {
		if (disposed || saving) return;
		const version = ++revision;
		saving = true;
		const response = await invoke<CommandResponse<WidgetLayout>>("reset_layout");
		if (disposed) return;
		saving = false;
		if (version !== revision) return;
		loading = false;
		if (response.status === "ready") {
			layout = response.data;
			error = null;
		} else error = response.message;
	}

	onMount(() => {
		const unlisten = listen("layout-changed", () => {
			void load();
		});
		void invoke<boolean>("island_available").then((available) => {
			if (disposed) return;
			islandAvailable = available === true;
		});
		void load();
		void invoke<CommandResponse<ServiceStatusCatalogEntry[]>>("read_service_catalog").then(
			(response) => {
				if (disposed) return;
				if (response.status === "ready") serviceCatalog = response.data;
				else serviceCatalogError = response.message;
			},
		);
		return () => {
			disposed = true;
			revision += 1;
			void unlisten.then((stop) => stop());
		};
	});

	return {
		get islandAvailable() {
			return islandAvailable;
		},
		get layout() {
			return layout;
		},
		get loading() {
			return loading;
		},
		get saving() {
			return saving;
		},
		get error() {
			return error;
		},
		get serviceCatalog() {
			return serviceCatalog;
		},
		get serviceCatalogError() {
			return serviceCatalogError;
		},
		load,
		save,
		reset,
	};
}
