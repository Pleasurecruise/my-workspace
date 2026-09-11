import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, UpdateInfo } from "../../../consumer";
import UpdateDialog from "../UpdateDialog.svelte";
const { invoke, stop } = vi.hoisted(() => ({ invoke: vi.fn(), stop: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => stop) }));
afterEach(() => {
	invoke.mockReset();
	stop.mockReset();
});

it("owns the update prompt and releases the shell after Later", async () => {
	invoke.mockResolvedValue({
		status: "ready",
		data: { version: "2.0.0", currentVersion: "1.0.0", notes: "Release notes" },
	});
	const target = document.createElement("div");
	const onmodalchange = vi.fn();
	const view = mount(UpdateDialog, { target, props: { locked: false, onmodalchange } });
	try {
		await vi.waitFor(() => expect(target.textContent).toContain("Vesper 2.0.0 is available"));
		expect(onmodalchange).toHaveBeenLastCalledWith(true);
		expect(invoke).not.toHaveBeenCalledWith("install_update", expect.anything());
		const later = Array.from(target.querySelectorAll("button")).find(
			(button) => button.textContent === "Later",
		);
		if (!later) throw new Error("Missing Later action");
		later.click();
		await tick();
		expect(target.querySelector('[role="dialog"]')).toBeNull();
		expect(onmodalchange).toHaveBeenLastCalledWith(false);
	} finally {
		await unmount(view);
	}
	expect(stop).toHaveBeenCalledTimes(2);
});

it("discards a check response after the component is destroyed", async () => {
	let finish: (response: CommandResponse<UpdateInfo | null>) => void = () => {
		throw new Error("Check not started");
	};
	const pending = new Promise<CommandResponse<UpdateInfo | null>>((resolve) => {
		finish = resolve;
	});
	invoke.mockReturnValue(pending);
	const target = document.createElement("div");
	const onmodalchange = vi.fn();
	const view = mount(UpdateDialog, { target, props: { locked: false, onmodalchange } });
	await tick();
	await unmount(view);
	finish({ status: "ready", data: { version: "2.0.0", currentVersion: "1.0.0", notes: null } });
	await tick();
	expect(onmodalchange).not.toHaveBeenCalledWith(true);
});
