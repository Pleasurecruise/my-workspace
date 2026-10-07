import { invoke } from "@tauri-apps/api/core";
import { onMount } from "svelte";
import type { CommandResponse } from "@/lib/contracts/command";
import type { NewspaperDaily } from "@/lib/contracts/newspaper";

export function createNewspaperSession(isActive: () => boolean) {
	let daily = $state<NewspaperDaily | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let request = 0;

	async function refresh() {
		if (loading) return;
		const version = ++request;
		loading = true;
		const response = await invoke<CommandResponse<NewspaperDaily>>("read_newspaper");
		if (version !== request) return;
		loading = false;
		if (response.status === "ready") {
			daily = response.data;
			error = null;
		} else error = response.message;
	}

	onMount(() => {
		const focus = () => {
			if (isActive()) void refresh();
		};
		window.addEventListener("focus", focus);
		return () => {
			request += 1;
			window.removeEventListener("focus", focus);
		};
	});

	return {
		get daily() {
			return daily;
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
