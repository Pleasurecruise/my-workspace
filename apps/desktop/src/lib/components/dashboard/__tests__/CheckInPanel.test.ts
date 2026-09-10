import { beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CheckIn, CommandResponse } from "../../../consumer";
import CheckInPanel from "../CheckInPanel.svelte";

const { invoke, listen, listeners, unlisten } = vi.hoisted(() => ({
	invoke: vi.fn(),
	listen: vi.fn(),
	unlisten: vi.fn(),
	listeners: new Map<string, (event: { payload: string }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
beforeEach(() => {
	invoke.mockReset();
	listen.mockReset();
	unlisten.mockReset();
	listeners.clear();
	listen.mockImplementation((name, callback) => {
		listeners.set(name, callback);
		return Promise.resolve(unlisten);
	});
});
const progress: CheckIn = {
	date: "2026-09-10",
	completed: false,
	streak: 2,
	total: 3,
	days: [
		{ date: "2026-09-09", completed: true },
		{ date: "2026-09-10", completed: false },
	],
};
async function settle() {
	await tick();
	await tick();
	await tick();
}

it("checks in once, coalesces window updates during the write, and supports undo", async () => {
	const checked: CheckIn = { ...progress, completed: true, streak: 3, total: 4 };
	const pending = deferred<CommandResponse<CheckIn>>();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: progress })
		.mockReturnValueOnce(pending.promise)
		.mockResolvedValueOnce({ status: "ready", data: checked })
		.mockResolvedValueOnce({ status: "ready", data: progress });
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(CheckInPanel, { target, props: { id: "read", name: "Read daily" } });
	try {
		await settle();
		const button = target.querySelector<HTMLButtonElement>(".check-button");
		if (button === null) throw new Error("Missing check-in button");
		button.click();
		button.click();
		await tick();
		expect(invoke).toHaveBeenCalledTimes(2);
		expect(invoke).toHaveBeenLastCalledWith("set_check_in", {
			id: "read",
			date: progress.date,
			completed: true,
		});
		listeners.get("check-in-updated")?.({ payload: "read" });
		listeners.get("check-in-updated")?.({ payload: "read" });
		expect(invoke).toHaveBeenCalledTimes(2);
		pending.resolve({ status: "ready", data: checked });
		await settle();
		expect(invoke).toHaveBeenCalledTimes(3);
		expect(button.getAttribute("aria-pressed")).toBe("true");
		button.click();
		await settle();
		expect(invoke).toHaveBeenLastCalledWith("set_check_in", {
			id: "read",
			date: progress.date,
			completed: false,
		});
		expect(button.getAttribute("aria-pressed")).toBe("false");
	} finally {
		await unmount(view);
		target.remove();
	}
	await Promise.resolve();
	expect(unlisten).toHaveBeenCalledTimes(1);
});

it("keeps settled progress on failure and ignores an older refresh response", async () => {
	const older = deferred<CommandResponse<CheckIn>>();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: progress })
		.mockResolvedValueOnce({ status: "failed", message: "Database unavailable" })
		.mockReturnValueOnce(older.promise)
		.mockResolvedValueOnce({ status: "ready", data: { ...progress, completed: true } });
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(CheckInPanel, { target, props: { id: "read", name: "Read daily" } });
	try {
		await settle();
		target.querySelector<HTMLButtonElement>(".check-button")?.click();
		await settle();
		expect(target.querySelector('[role="alert"]')?.textContent).toBe("Database unavailable");
		expect(target.querySelector(".check-button")?.getAttribute("aria-pressed")).toBe("false");
		window.dispatchEvent(new Event("focus"));
		listeners.get("check-in-updated")?.({ payload: "read" });
		await settle();
		older.resolve({ status: "ready", data: progress });
		await settle();
		expect(target.querySelector(".check-button")?.getAttribute("aria-pressed")).toBe("true");
	} finally {
		await unmount(view);
		target.remove();
	}
});

function deferred<T>() {
	let resolve: (value: T) => void = () => {
		throw new Error("Promise is not initialized");
	};
	const promise = new Promise<T>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}

it("retains a failed check-in error after an update arrives during the write", async () => {
	const pending = deferred<CommandResponse<CheckIn>>();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: progress })
		.mockReturnValueOnce(pending.promise)
		.mockResolvedValueOnce({ status: "ready", data: progress });
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(CheckInPanel, { target, props: { id: "read", name: "Read daily" } });
	try {
		await settle();
		target.querySelector<HTMLButtonElement>(".check-button")?.click();
		listeners.get("check-in-updated")?.({ payload: "read" });
		pending.resolve({ status: "failed", message: "Could not save check-in" });
		await settle();
		expect(target.querySelector('[role="alert"]')?.textContent).toBe("Could not save check-in");
		invoke.mockResolvedValueOnce({ status: "ready", data: progress });
		window.dispatchEvent(new Event("focus"));
		await settle();
		expect(target.querySelector('[role="alert"]')?.textContent).toBe("Could not save check-in");
		invoke.mockResolvedValueOnce({ status: "ready", data: { ...progress, completed: true } });
		target.querySelector<HTMLButtonElement>(".check-button")?.click();
		await settle();
		expect(target.querySelector('[role="alert"]')).toBeNull();
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("removes listeners and never rereads after unmounting with a write and an update pending", async () => {
	const pending = deferred<CommandResponse<CheckIn>>();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: progress })
		.mockReturnValueOnce(pending.promise);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(CheckInPanel, { target, props: { id: "read", name: "Read daily" } });
	await settle();
	target.querySelector<HTMLButtonElement>(".check-button")?.click();
	listeners.get("check-in-updated")?.({ payload: "read" });
	await unmount(view);
	target.remove();
	pending.resolve({ status: "ready", data: { ...progress, completed: true } });
	window.dispatchEvent(new Event("focus"));
	listeners.get("check-in-updated")?.({ payload: "read" });
	await settle();
	expect(unlisten).toHaveBeenCalledTimes(1);
	expect(invoke).toHaveBeenCalledTimes(2);
});
