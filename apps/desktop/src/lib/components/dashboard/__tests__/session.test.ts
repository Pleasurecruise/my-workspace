import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";
import type { CommandResponse, TodoList } from "../../../consumer";
import { createDashboardSession } from "../session.svelte";

const { invoke, listen, mounts, listeners } = vi.hoisted(() => ({
	invoke: vi.fn(),
	listen: vi.fn(),
	mounts: [] as Array<() => () => void>,
	listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("svelte", async (original) => ({
	...(await original<typeof import("svelte")>()),
	onMount: (callback: () => () => void) => mounts.push(callback),
}));
let cleanup: (() => void) | undefined;
beforeEach(() => {
	invoke.mockReset();
	listen.mockReset();
	mounts.length = 0;
	listeners.clear();
	listen.mockImplementation((name, callback) => {
		listeners.set(name, callback);
		return Promise.resolve(() => listeners.delete(name));
	});
});
afterEach(() => {
	cleanup?.();
	cleanup = undefined;
});

it("rejects an older date response after switching dates", async () => {
	const session = createDashboardSession(() => false, "island");
	let resolve!: (response: CommandResponse<TodoList>) => void;
	const first = new Promise<CommandResponse<TodoList>>((settle) => {
		resolve = settle;
	});
	invoke.mockReturnValueOnce(first).mockResolvedValueOnce({
		status: "ready",
		data: { date: "2026-09-08", items: [], syncError: null },
	});
	const older = session.loadTodos("2026-09-07");
	await session.loadTodos("2026-09-08");
	resolve({ status: "ready", data: { date: "2026-09-07", items: [], syncError: null } });
	await older;
	expect(session.todoDate).toBe("2026-09-08");
	expect(session.todos.data?.date).toBe("2026-09-08");
});

it("coalesces cross-window updates during a read and retains API failure feedback", async () => {
	const session = createDashboardSession(() => false, "island");
	cleanup = mounts[0]!();
	let resolve!: (response: CommandResponse<TodoList>) => void;
	const pending = new Promise<CommandResponse<TodoList>>((settle) => {
		resolve = settle;
	});
	invoke.mockReturnValueOnce(pending).mockResolvedValueOnce({
		status: "ready",
		data: { date: "2026-09-07", items: [], syncError: "Notion unavailable" },
	});
	const read = session.loadTodos("2026-09-07");
	listeners.get("todo-updated")?.({ payload: "2026-09-08" });
	listeners.get("todo-updated")?.({ payload: "2026-09-07" });
	listeners.get("todo-updated")?.({ payload: "2026-09-07" });
	expect(invoke).toHaveBeenCalledTimes(1);
	resolve({ status: "ready", data: { date: "2026-09-07", items: [], syncError: null } });
	await read;
	await Promise.resolve();
	expect(invoke).toHaveBeenCalledTimes(2);
	expect(session.todos.error).toBe("Notion unavailable");
	expect(session.todos.loading).toBe(false);
});

it("does not replay an invalidated read after the island session is disposed", async () => {
	const session = createDashboardSession(() => false, "island");
	cleanup = mounts[0]!();
	let resolve!: (response: CommandResponse<TodoList>) => void;
	const pending = new Promise<CommandResponse<TodoList>>((settle) => {
		resolve = settle;
	});
	invoke.mockReturnValueOnce(pending);
	const read = session.loadTodos("2026-09-07");
	listeners.get("todo-updated")?.({ payload: "2026-09-07" });
	cleanup();
	cleanup = undefined;
	resolve({ status: "ready", data: { date: "2026-09-07", items: [], syncError: null } });
	await read;
	expect(invoke).toHaveBeenCalledTimes(1);
	expect(session.todos.data).toBeNull();
});
