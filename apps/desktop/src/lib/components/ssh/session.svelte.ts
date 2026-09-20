import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type { CommandResponse, SshDevice, SshSnapshot } from "../../consumer";

export function createSshSession(isLocked: () => boolean) {
	let devices = $state<SshDevice[]>([]);
	let openDevices = $state<SshDevice[]>([]);
	let selectedId = $state<string | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let mounted = $state(false);
	let disposed = false;
	let generation = 0;

	function applySnapshot(snapshot: SshSnapshot) {
		devices = snapshot.devices;
		error = snapshot.error;
		if (snapshot.error === null) {
			openDevices = openDevices.filter((device) =>
				snapshot.devices.some((next) => next.id === device.id),
			);
			if (!openDevices.some((device) => device.id === selectedId))
				selectedId = openDevices[0]?.id ?? null;
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
		void invoke<CommandResponse<null>>("set_ssh_active", { active })
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
			void invoke("set_ssh_active", { active: false });
		};
	});
	return {
		get devices() {
			return devices;
		},
		get openDevices() {
			return openDevices;
		},
		get selectedId() {
			return selectedId;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		refresh,
		open(device: SshDevice) {
			const next = { ...device };
			openDevices = [...openDevices.filter((item) => item.id !== device.id), next];
			selectedId = device.id;
		},
	};
}
