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
		return () => listeners.delete(name);
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
		if (command === "island_available") return true;
		if (command === "set_island_expanded")
			return { status: "ready", data: { topInset: 0, notchWidth: 0 } };
		if (command === "read_service_status_catalog") return { status: "ready", data: [] };
		if (command === "read_todos")
			return { status: "ready", data: { date: "2026-09-07", items: [], syncError: null } };
		return { status: "failed", message: "Unexpected command" };
	});
});
afterEach(() => document.body.replaceChildren());

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
