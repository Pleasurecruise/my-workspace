import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type { CommandResponse, SshDevice, SshSnapshot, TerminalTarget } from "../../consumer";

export function createTerminalSession(isLocked: () => boolean) {
	let devices = $state<SshDevice[]>([]);
	let openTerminals = $state.raw<TerminalTarget[]>([]);
	let selected = $state.raw<TerminalTarget | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let mounted = $state(false);
	let disposed = false;
	let generation = 0;

	function applySnapshot(snapshot: SshSnapshot) {
		devices = snapshot.devices;
		error = snapshot.error;
		if (snapshot.error === null) {
			openTerminals = openTerminals.filter(
				(target) =>
					target.kind === "local" || snapshot.devices.some((next) => next.id === target.device.id),
			);
			if (selected !== null && !openTerminals.includes(selected)) {
				const [first] = openTerminals;
				selected = first === undefined ? null : first;
			}
		}
	}
	async function refresh() {
		if (loading || disposed || isLocked()) return;
		const version = generation;
		loading = true;
		try {
			const response = await invoke<CommandResponse<SshSnapshot>>("read_ssh_devices");
			if (disposed || version !== generation) return;
			if (response.status === "ready") applySnapshot(response.data);
			else error = response.message;
		} catch {
			if (!disposed && version === generation)
				error = "Could not read Tailscale devices. Try refreshing.";
		} finally {
			if (version === generation) loading = false;
		}
	}
	$effect(() => {
		if (!mounted) return;
		const active = !isLocked();
		const version = ++generation;
		loading = false;
		void invoke<CommandResponse<null>>("set_terminal_active", { active })
			.then((response) => {
				if (disposed || version !== generation) return;
				if (response.status === "failed") error = response.message;
				else if (active) void refresh();
			})
			.catch(() => {
				if (!disposed && version === generation) error = "Could not activate Tailscale discovery.";
			});
	});
	onMount(() => {
		mounted = true;
		const unlisten = listen<SshSnapshot>("ssh-devices-changed", (event) => {
			if (!disposed && !isLocked()) applySnapshot(event.payload);
		});
		return () => {
			disposed = true;
			generation += 1;
			void unlisten.then((stop) => stop());
			void invoke("set_terminal_active", { active: false });
		};
	});
	return {
		get devices() {
			return devices;
		},
		get openTerminals() {
			return openTerminals;
		},
		get selected() {
			return selected;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		refresh,
		open(target: TerminalTarget) {
			const next = { ...target };
			openTerminals = [
				...openTerminals.filter(
					(item) =>
						item.kind !== target.kind ||
						(item.kind === "ssh" && target.kind === "ssh" && item.device.id !== target.device.id),
				),
				next,
			];
			selected = next;
		},
	};
}
