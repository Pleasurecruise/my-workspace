import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CheckIn, CommandResponse } from "../../consumer";

export function createCheckInSession(getId: () => string) {
	let data = $state<CheckIn | null>(null);
	let error = $state<string | null>(null);
	let writeError = $state<string | null>(null);
	let loading = $state(false);
	let saving = false;
	let invalidated = false;
	let request = 0;

	async function refresh(id: string) {
		if (saving) {
			invalidated = true;
			return;
		}
		const version = ++request;
		loading = true;
		const response = await invoke<CommandResponse<CheckIn>>("read_check_in", { id });
		if (version !== request) return;
		loading = false;
		if (response.status === "ready") {
			if (data !== null && data.date !== response.data.date) writeError = null;
			data = response.data;
			error = null;
		} else error = response.message;
	}

	async function toggle() {
		if (loading || data === null) return;
		const id = getId();
		const version = ++request;
		loading = true;
		saving = true;
		error = null;
		writeError = null;
		const response = await invoke<CommandResponse<CheckIn>>("set_check_in", {
			id,
			date: data.date,
			completed: !data.completed,
		});
		if (version !== request) return;
		saving = false;
		loading = false;
		if (response.status === "ready") data = response.data;
		else writeError = response.message;
		if (invalidated) {
			invalidated = false;
			void refresh(id);
		}
	}

	$effect(() => {
		const id = getId();
		data = null;
		error = null;
		writeError = null;
		saving = false;
		invalidated = false;
		let disposed = false;
		const listener = listen<string>("check-in-updated", (event) => {
			if (!disposed && event.payload === id) void refresh(id);
		});
		void listener.then(() => {
			if (!disposed) void refresh(id);
		});
		// Refresh after sleep, date/time-zone changes, and updates from other windows.
		const timer = setInterval(() => void refresh(id), 60_000);
		const onFocus = () => void refresh(id);
		window.addEventListener("focus", onFocus);
		return () => {
			disposed = true;
			++request;
			clearInterval(timer);
			window.removeEventListener("focus", onFocus);
			void listener.then((unlisten) => unlisten());
		};
	});

	return {
		get data() {
			return data;
		},
		get error() {
			return writeError === null ? error : writeError;
		},
		get loading() {
			return loading;
		},
		toggle,
		refresh: () => refresh(getId()),
	};
}
