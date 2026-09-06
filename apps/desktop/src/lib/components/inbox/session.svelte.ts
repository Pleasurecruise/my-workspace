import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type { CommandResponse, NtfyNotification } from "../../consumer";

export function createInboxSession(isActive: () => boolean) {
	let notifications = $state<NtfyNotification[]>([]);
	let notificationsError = $state<string | null>(null);
	async function markNotificationRead(id: string) {
		const response = await invoke<CommandResponse<NtfyNotification[]>>("mark_notification_read", {
			id,
		});
		if (response.status === "ready") notifications = response.data;
		return response;
	}

	async function activate(active: boolean) {
		const response = await invoke<CommandResponse<null>>("set_notifications_active", { active });
		if (isActive() && response.status === "failed") notificationsError = response.message;
	}
	onMount(() => {
		void invoke<CommandResponse<NtfyNotification[]>>("read_notifications").then((response) => {
			if (response.status === "ready") notifications = response.data;
			else notificationsError = response.message;
		});
		const unlistenNotifications = listen<NtfyNotification[]>("notifications-updated", (event) => {
			notifications = event.payload;
		});
		return () => {
			void invoke<CommandResponse<null>>("set_notifications_active", { active: false });
			void unlistenNotifications.then((unlisten) => unlisten());
		};
	});
	return {
		get notifications() {
			return notifications;
		},
		get error() {
			return notificationsError;
		},
		activate,
		markNotificationRead,
	};
}
