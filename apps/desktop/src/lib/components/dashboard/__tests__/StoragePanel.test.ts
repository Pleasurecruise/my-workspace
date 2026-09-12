import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import StoragePanel from "../StoragePanel.svelte";

const commands = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => commands);

it("reads no files on mount and opens system settings only on request", async () => {
	commands.invoke.mockResolvedValue({ status: "failed", message: "Settings unavailable" });
	const target = document.createElement("div");
	document.body.append(target);
	const props = {
		storage: { usedBytes: 50e9, totalBytes: 100e9, usedPercent: 50, sampledAt: 1 },
		error: null,
	};
	let view = mount(StoragePanel, { target, props });
	await tick();
	expect(commands.invoke).not.toHaveBeenCalled();
	expect(target.textContent).toContain("50 GB");
	await unmount(view);
	view = mount(StoragePanel, { target, props });
	await tick();
	expect(commands.invoke).not.toHaveBeenCalled();
	const button = target.querySelector("button");
	if (!button) throw new Error("Missing storage settings button");
	button.click();
	await vi.waitFor(() => expect(target.textContent).toContain("Settings unavailable"));
	expect(commands.invoke).toHaveBeenCalledExactlyOnceWith("open_storage_settings");
	expect(target.textContent).toContain("50 GB");
	commands.invoke.mockResolvedValue({ status: "ready", data: null });
	button.click();
	await vi.waitFor(() => expect(button.disabled).toBe(false));
	expect(target.querySelector('[role="alert"]')).toBeNull();
	await unmount(view);
	target.remove();
});
