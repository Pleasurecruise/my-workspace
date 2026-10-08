import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type { ChatSnapshot } from "@/lib/contracts/chat";
import type { CommandResponse } from "@/lib/contracts/command";

export function createChatSession() {
	let snapshot = $state<ChatSnapshot>({
		revision: 0,
		connected: false,
		busy: false,
		model: "",
		messages: [],
		error: null,
	});
	let draft = $state("");
	let connecting = $state(false);
	let pending = $state(false);
	let stopping = $state(false);
	let error = $state<string | null>(null);
	let disposed = false;
	let initialized = false;
	let generation = 0;

	function accept(next: ChatSnapshot) {
		if (disposed || (initialized && next.revision <= snapshot.revision)) return;
		initialized = true;
		snapshot = next;
	}

	async function refresh() {
		const version = generation;
		const response = await invoke<CommandResponse<ChatSnapshot>>("read_chat").catch(
			(): CommandResponse<ChatSnapshot> => ({
				status: "failed",
				message: "Could not read Pi status.",
			}),
		);
		if (disposed || version !== generation) return;
		if (response.status === "ready") accept(response.data);
		else error = response.message;
	}

	async function connect() {
		if (connecting) return;
		const version = generation;
		connecting = true;
		error = null;
		const response = await invoke<CommandResponse<ChatSnapshot>>("connect_chat").catch(
			(): CommandResponse<ChatSnapshot> => ({
				status: "failed",
				message: "Could not connect to Pi.",
			}),
		);
		if (disposed || version !== generation) return;
		connecting = false;
		if (response.status === "ready") accept(response.data);
		else error = response.message;
	}

	async function send() {
		if (pending || snapshot.busy || !snapshot.connected || draft.trim() === "") return;
		const version = generation;
		const submitted = draft;
		pending = true;
		error = null;
		const response = await invoke<CommandResponse<null>>("send_chat", { message: submitted }).catch(
			(): CommandResponse<null> => ({
				status: "failed",
				message: "Could not send the message to Pi.",
			}),
		);
		if (disposed || version !== generation) return;
		pending = false;
		if (response.status === "ready") {
			if (draft === submitted) draft = "";
		} else error = response.message;
	}

	async function control(action: "stop" | "newSession" | "disconnect") {
		if (pending && action !== "stop") return;
		if (stopping && action === "stop") return;
		const version = generation;
		error = null;
		if (action === "stop") stopping = true;
		const response = await invoke<CommandResponse<null>>("control_chat", { action }).catch(
			(): CommandResponse<null> => ({
				status: "failed",
				message: "Could not update the Pi session.",
			}),
		);
		if (!disposed && version === generation) {
			if (action === "stop") stopping = false;
			if (response.status === "failed") error = response.message;
		}
	}

	async function leave() {
		generation += 1;
		draft = "";
		error = null;
		pending = false;
		stopping = false;
		connecting = false;
		const response = await invoke<CommandResponse<null>>("control_chat", {
			action: "disconnect",
		}).catch((): CommandResponse<null> => ({ status: "failed", message: "Could not close Pi." }));
		if (response.status === "failed") error = response.message;
		await refresh();
	}

	onMount(() => {
		const registration = listen<ChatSnapshot>("chat-updated", (event) => accept(event.payload));
		void registration.then(() => {
			if (!disposed) void refresh();
		});
		return () => {
			disposed = true;
			void registration.then((stop) => stop());
		};
	});

	return {
		get snapshot() {
			return snapshot;
		},
		get pending() {
			return pending;
		},
		get stopping() {
			return stopping;
		},
		get error() {
			return error;
		},
		get draft() {
			return draft;
		},
		set draft(value: string) {
			draft = value;
		},
		get connecting() {
			return connecting;
		},
		refresh,
		connect,
		send,
		control,
		leave,
	};
}
