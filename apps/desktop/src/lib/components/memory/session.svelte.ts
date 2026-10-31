import { invoke } from "@tauri-apps/api/core";
import type { CommandResponse } from "@/lib/contracts/command";
import type { MemoryGraph } from "@/lib/contracts/nowledge";

export function createMemorySession() {
	let graph = $state.raw<MemoryGraph | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let request = 0;

	async function refresh() {
		const version = ++request;
		loading = true;
		const response = await invoke<CommandResponse<MemoryGraph>>("read_memory_graph");
		if (version !== request) return;
		loading = false;
		if (response.status === "ready") {
			graph = response.data;
			error = null;
		} else error = response.message;
	}

	return {
		get graph() {
			return graph;
		},
		get error() {
			return error;
		},
		get loading() {
			return loading;
		},
		refresh,
	};
}
