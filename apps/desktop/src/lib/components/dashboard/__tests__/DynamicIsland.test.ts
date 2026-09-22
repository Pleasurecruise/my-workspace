import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import IslandApp from "../../../../IslandApp.svelte";
import type { CommandResponse } from "../../../consumer";

const { invoke, listeners } = vi.hoisted(() => ({
	invoke: vi.fn(),
	listeners: new Map<string, (event: { payload: string }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (path: string) => path }));
vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn(async (name: string, handler: (event: { payload: string }) => void) => {
		listeners.set(name, handler);
		return () => {
			if (listeners.get(name) === handler) listeners.delete(name);
		};
	}),
}));

beforeEach(() => {
	listeners.clear();
	invoke.mockReset();
	invoke.mockImplementation(async (command: string) => {
		if (command === "read_layout")
			return {
				status: "ready",
				data: {
					widgets: [{ id: "todo", widget: { kind: "planner", habits: [] } }],
					islandWidgetId: "todo",
				},
			};
		if (command === "read_planner_date") return { status: "ready", data: "2026-09-07" };
		if (command === "island_available") return true;
		if (command === "set_island_expanded")
			return { status: "ready", data: { topInset: 0, notchWidth: 0 } };
		if (command === "read_service_catalog") return { status: "ready", data: [] };
		if (command === "read_todos")
			return { status: "ready", data: { date: "2026-09-07", items: [], syncError: null } };
		return { status: "failed", message: "Unexpected command" };
	});
});
afterEach(() => {
	vi.useRealTimers();
	document.body.replaceChildren();
});

it("opens Todo without starting Dashboard polling and collapses on Escape", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	const trigger = target.querySelector<HTMLButtonElement>(".trigger");
	trigger?.click();
	await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("read_todos", expect.anything()));
	expect(target.querySelector(".todo")).not.toBeNull();
	expect(invoke).not.toHaveBeenCalledWith("set_dashboard_active", expect.anything());
	window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
	await tick();
	expect(target.querySelector(".todo")).toBeNull();
	expect(document.activeElement).toBe(target.querySelector(".trigger"));
	await unmount(view);
	await vi.waitFor(() => expect(listeners.size).toBe(0));
});

it("does not execute a queued expansion after the island is destroyed", async () => {
	let release = () => {};
	const pending = new Promise<CommandResponse<{ topInset: number; notchWidth: number }>>(
		(resolve) => {
			release = () => resolve({ status: "ready", data: { topInset: 0, notchWidth: 0 } });
		},
	);
	// Hold the first native resize, leaving expansion queued behind it.
	const normal = invoke.getMockImplementation();
	invoke.mockImplementation((command: string) => {
		if (command === "set_island_expanded") return pending;
		return normal?.(command);
	});
	const target = document.createElement("div");
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	target.querySelector<HTMLButtonElement>(".trigger")?.click();
	await tick();
	await unmount(view);
	release();
	await tick();
	await tick();
	expect(invoke).not.toHaveBeenCalledWith("set_island_expanded", { expanded: true });
	expect(invoke).not.toHaveBeenCalledWith("read_todos", expect.anything());
});

it("ignores a passing pointer and lets Escape cancel a pending hover", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	vi.useFakeTimers();
	const island = target.querySelector<HTMLElement>(".island");
	if (island === null) throw new Error("Dynamic Island did not mount");
	island.dispatchEvent(new PointerEvent("pointerenter", { pointerType: "mouse" }));
	await vi.advanceTimersByTimeAsync(100);
	island.dispatchEvent(new PointerEvent("pointerleave", { pointerType: "mouse" }));
	await vi.advanceTimersByTimeAsync(300);
	expect(invoke).not.toHaveBeenCalledWith("set_island_expanded", { expanded: true });
	island.dispatchEvent(new PointerEvent("pointerenter", { pointerType: "mouse" }));
	window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
	await vi.advanceTimersByTimeAsync(200);
	expect(invoke).not.toHaveBeenCalledWith("set_island_expanded", { expanded: true });
	await unmount(view);
});

it("keeps the island open when the pointer returns during the leave grace period", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	vi.useFakeTimers();
	const island = target.querySelector<HTMLElement>(".island");
	if (island === null) throw new Error("Dynamic Island did not mount");
	island.dispatchEvent(new PointerEvent("pointerenter", { pointerType: "mouse" }));
	await vi.advanceTimersByTimeAsync(160);
	expect(target.querySelector(".expanded-view")).not.toBeNull();
	island.dispatchEvent(new PointerEvent("pointerleave", { pointerType: "mouse" }));
	await vi.advanceTimersByTimeAsync(200);
	island.dispatchEvent(new PointerEvent("pointerenter", { pointerType: "mouse" }));
	await vi.advanceTimersByTimeAsync(300);
	expect(target.querySelector(".expanded-view")).not.toBeNull();
	island.dispatchEvent(new PointerEvent("pointerleave", { pointerType: "mouse" }));
	await vi.advanceTimersByTimeAsync(280);
	expect(target.querySelector(".expanded-view")).toBeNull();
	await unmount(view);
});

it("retains keyboard focus on explicit activation and cancels hover on disposal", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	target.querySelector<HTMLButtonElement>(".trigger")?.click();
	await vi.waitFor(() => expect(document.activeElement).toBe(target.querySelector(".close")));
	vi.useFakeTimers();
	target.querySelector(".island")?.dispatchEvent(new PointerEvent("pointerleave"));
	await vi.advanceTimersByTimeAsync(300);
	expect(target.querySelector(".expanded-view")).not.toBeNull();
	window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
	await tick();
	target
		.querySelector(".island")
		?.dispatchEvent(new PointerEvent("pointerenter", { pointerType: "mouse" }));
	invoke.mockClear();
	await unmount(view);
	await vi.advanceTimersByTimeAsync(300);
	expect(invoke).not.toHaveBeenCalledWith("set_island_expanded", { expanded: true });
});

it("opens on Tasks and preserves its draft while browsing Calendar and Habits", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	target.querySelector<HTMLButtonElement>(".trigger")?.click();
	await vi.waitFor(() => expect(target.querySelector(".todo input")).not.toBeNull());
	const input = target.querySelector<HTMLInputElement>(".todo input");
	if (input === null) throw new Error("Task input did not mount");
	input.value = "Keep this draft";
	input.dispatchEvent(new Event("input", { bubbles: true }));
	const sections = Array.from(
		target.querySelectorAll<HTMLButtonElement>("nav[aria-label='Planner sections'] button"),
	);
	expect(sections[0]?.getAttribute("aria-pressed")).toBe("true");
	expect(target.querySelector(".compact-toolbar time")?.textContent).toContain("2026-09-07");
	sections[1]?.click();
	await tick();
	expect(target.querySelector<HTMLElement>(".planner-todos")?.hidden).toBe(true);
	expect(target.querySelector<HTMLElement>(".planner-calendar")?.hidden).toBe(false);
	sections[2]?.click();
	await tick();
	expect(target.querySelector<HTMLElement>(".planner-habits")?.hidden).toBe(false);
	sections[0]?.click();
	await tick();
	expect(target.querySelector<HTMLElement>(".planner-todos")?.hidden).toBe(false);
	expect(target.querySelector<HTMLInputElement>(".todo input")?.value).toBe("Keep this draft");
	await unmount(view);
});

it("does not reveal content or steal focus after focus loss during native expansion", async () => {
	let release = () => {};
	const pending = new Promise<CommandResponse<{ topInset: number; notchWidth: number }>>(
		(resolve) => {
			release = () => resolve({ status: "ready", data: { topInset: 0, notchWidth: 0 } });
		},
	);
	const normal = invoke.getMockImplementation();
	invoke.mockImplementation((command: string, args: { expanded?: boolean }) => {
		if (command === "set_island_expanded" && args.expanded) return pending;
		return normal?.(command, args);
	});
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(IslandApp, { target });
	await vi.waitFor(() => expect(target.querySelector(".trigger")).not.toBeNull());
	target.querySelector<HTMLButtonElement>(".trigger")?.click();
	await vi.waitFor(() =>
		expect(invoke).toHaveBeenCalledWith("set_island_expanded", { expanded: true }),
	);
	expect(listeners.has("island-collapse")).toBe(true);
	listeners.get("island-collapse")?.({ payload: "" });
	release();
	await tick();
	await tick();
	expect(target.querySelector(".expanded-view")).toBeNull();
	expect(invoke).not.toHaveBeenCalledWith("read_todos", expect.anything());
	expect(document.activeElement).toBe(document.body);
	await unmount(view);
});
