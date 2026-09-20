import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, unmount } from "svelte";
import DeviceSidebar from "../DeviceSidebar.svelte";
import type { createTerminalSession } from "../session.svelte";

const views: ReturnType<typeof mount>[] = [];
afterEach(async () => {
	for (const view of views.splice(0)) await unmount(view);
	document.body.replaceChildren();
});
it("distinguishes online, offline and unavailable status and opens the chosen device", () => {
	const devices = [true, false, null].map((online, index) => ({
		id: String(index),
		name: `device-${index}`,
		dnsName: "",
		address: `100.64.0.${index + 1}`,
		os: "linux",
		online,
		username: "user",
	}));
	const onopen = vi.fn();
	const refresh = vi.fn();
	const session: ReturnType<typeof createTerminalSession> = {
		devices,
		openTerminals: [],
		selected: null,
		loading: false,
		error: null,
		refresh,
		open: vi.fn(),
	};
	views.push(
		mount(DeviceSidebar, {
			target: document.body,
			props: { session, compact: false, active: false, onopen },
		}),
	);
	const buttons = [...document.querySelectorAll("button")];
	for (const status of ["Online", "Offline", "Status unknown"]) {
		expect(buttons.some((button) => button.getAttribute("aria-label")?.includes(status))).toBe(
			true,
		);
	}
	const deviceButton = buttons.find((button) =>
		button.getAttribute("aria-label")?.startsWith("device-2,"),
	);
	if (deviceButton === undefined) throw new Error("Expected device button");
	deviceButton.click();
	expect(onopen).toHaveBeenCalledWith({ kind: "ssh", device: devices[2] });
	const refreshButton = buttons.find(
		(button) => button.getAttribute("aria-label") === "Refresh Tailscale devices",
	);
	if (refreshButton === undefined) throw new Error("Expected refresh button");
	refreshButton.click();
	expect(refresh).toHaveBeenCalledOnce();
});
