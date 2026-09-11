import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import HabitsPanel from "../HabitsPanel.svelte";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
it("checks in an individual habit without selection controls and keeps write errors visible", async () => {
	const state = { date: "2026-09-11", completed: false, streak: 0, total: 0, days: [] };
	invoke.mockImplementation(async (command: string) =>
		command === "read_check_ins"
			? { status: "ready", data: [state, state] }
			: { status: "failed", message: "Write failed" },
	);
	const onchange = vi.fn(async () => true);
	const habits = [
		{ id: "read", name: "Read" },
		{ id: "walk", name: "Walk" },
	];
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(HabitsPanel, { target, props: { habits, onchange } });
	const button = (text: string) => {
		const found = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((item) =>
			item.getAttribute("aria-label")?.startsWith(text),
		);
		if (!found) throw new Error(`Missing ${text}`);
		return found;
	};
	try {
		await vi.waitFor(() => expect(button("Check in").disabled).toBe(false));
		expect(target.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
		expect(target.querySelector('button[aria-label="Select habits"]')).toBeNull();
		button("Check in Read").click();
		await vi.waitFor(() => expect(target.textContent).toContain("Write failed"));
		expect(invoke).toHaveBeenCalledWith("set_check_in", {
			id: "read",
			date: state.date,
			completed: true,
		});
		window.dispatchEvent(new Event("focus"));
		await tick();
		await tick();
		expect(target.textContent).toContain("Write failed");
		button("Manage").click();
		await tick();
		const input = target.querySelector("textarea");
		if (!input) throw new Error("Missing habit editor");
		input.value = "Exercise, Drink water";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		await tick();
		target.querySelector("form")?.dispatchEvent(new Event("submit", { cancelable: true }));
		await tick();
		expect(onchange).toHaveBeenCalledWith([
			...habits,
			{ id: expect.any(String), name: "Exercise" },
			{ id: expect.any(String), name: "Drink water" },
		]);
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("keeps a successful check-in when the following read fails and can undo it", async () => {
	const state = { date: "2026-09-11", completed: false, streak: 0, total: 0, days: [] };
	const checked = { ...state, completed: true, streak: 1, total: 1 };
	invoke.mockReset();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: [state] })
		.mockResolvedValueOnce({ status: "ready", data: checked })
		.mockResolvedValueOnce({ status: "failed", message: "Read failed" })
		.mockResolvedValueOnce({ status: "ready", data: state })
		.mockResolvedValueOnce({ status: "ready", data: [state] });
	const target = document.createElement("div");
	const view = mount(HabitsPanel, { target, props: { habits: [{ id: "read", name: "Read" }] } });
	try {
		await vi.waitFor(() =>
			expect(
				target.querySelector<HTMLButtonElement>('[aria-label="Check in Read"]')?.disabled,
			).toBe(false),
		);
		target.querySelector<HTMLButtonElement>('[aria-label="Check in Read"]')?.click();
		await vi.waitFor(() => expect(target.textContent).toContain("Read failed"));
		const undo = target.querySelector<HTMLButtonElement>('[aria-label="Undo Read"]');
		expect(undo?.getAttribute("aria-pressed")).toBe("true");
		undo?.click();
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("set_check_in", {
				id: "read",
				date: state.date,
				completed: false,
			}),
		);
	} finally {
		await unmount(view);
	}
});

it("preserves text entered while saving a habit", async () => {
	invoke.mockReset();
	invoke.mockResolvedValue({ status: "ready", data: [] });
	let finish = (_result: boolean) => {};
	const pending = new Promise<boolean>((resolve) => {
		finish = resolve;
	});
	const onchange = vi.fn(() => pending);
	const target = document.createElement("div");
	const view = mount(HabitsPanel, { target, props: { habits: [], onchange } });
	try {
		await tick();
		target.querySelector<HTMLButtonElement>('[aria-label="Manage habits"]')?.click();
		await tick();
		const input = target.querySelector("textarea");
		if (input === null) throw new Error("Missing editor");
		input.value = "Read";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		target.querySelector("form")?.dispatchEvent(new Event("submit", { cancelable: true }));
		await tick();
		input.value = "Exercise";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		finish(true);
		await tick();
		await tick();
		expect(input.value).toBe("Exercise");
		expect(onchange).toHaveBeenCalledWith([{ id: expect.any(String), name: "Read" }]);
	} finally {
		await unmount(view);
	}
});

it("rejects late reads and stops focus reads after unmount", async () => {
	const state = { date: "2026-09-11", completed: false, streak: 0, total: 0, days: [] };
	let finish = (_result: { status: "ready"; data: (typeof state)[] }) => {};
	const pending = new Promise<{ status: "ready"; data: (typeof state)[] }>((resolve) => {
		finish = resolve;
	});
	invoke.mockReset();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: [state] })
		.mockReturnValueOnce(pending)
		.mockResolvedValueOnce({ status: "ready", data: [{ ...state, completed: true }] });
	const target = document.createElement("div");
	const view = mount(HabitsPanel, { target, props: { habits: [{ id: "read", name: "Read" }] } });
	await vi.waitFor(() =>
		expect(target.querySelector<HTMLButtonElement>('[aria-label="Check in Read"]')?.disabled).toBe(
			false,
		),
	);
	window.dispatchEvent(new Event("focus"));
	window.dispatchEvent(new Event("focus"));
	await vi.waitFor(() => expect(target.querySelector('[aria-label="Undo Read"]')).not.toBeNull());
	finish({ status: "ready", data: [state] });
	await tick();
	await tick();
	expect(target.querySelector('[aria-label="Undo Read"]')).not.toBeNull();
	await unmount(view);
	const calls = invoke.mock.calls.length;
	window.dispatchEvent(new Event("focus"));
	expect(invoke).toHaveBeenCalledTimes(calls);
});
