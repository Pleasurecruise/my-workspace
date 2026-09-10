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

it("passes Todo descriptions and ignores an edit response after date navigation", async () => {
	const session = createDashboardSession(() => false, "island");
	invoke.mockResolvedValueOnce({
		status: "ready",
		data: { date: "2026-09-10", items: [], syncError: null },
	});
	await session.loadTodos("2026-09-10");
	const pending = deferred<CommandResponse<TodoList>>();
	invoke.mockReturnValueOnce(pending.promise);
	const edit = session.editTodo("read", "Read", "Chapter two");
	expect(invoke).toHaveBeenLastCalledWith("update_todo", {
		date: "2026-09-10",
		id: "read",
		text: "Read",
		description: "Chapter two",
	});
	invoke.mockResolvedValueOnce({
		status: "ready",
		data: { date: "2026-09-11", items: [], syncError: null },
	});
	await session.loadTodos("2026-09-11");
	pending.resolve({ status: "ready", data: { date: "2026-09-10", items: [], syncError: null } });
	expect(await edit).toBe(false);
	await vi.waitFor(() => expect(session.todos.data?.date).toBe("2026-09-11"));
	invoke.mockResolvedValueOnce({ status: "failed", message: "Database unavailable" });
	expect(await session.addTodo("Walk", "Around the park")).toBe(false);
	expect(invoke).toHaveBeenLastCalledWith("add_todo", {
		date: "2026-09-11",
		text: "Walk",
		description: "Around the park",
	});
	expect(session.todos.error).toBe("Database unavailable");
});

it("defers same-date refreshes until an edit commits", async () => {
	const session = createDashboardSession(() => false, "island");
	const before: TodoList = {
		date: "2026-09-10",
		syncError: null,
		items: [{ id: "read", text: "Read", description: null, completed: true, details: null }],
	};
	const after: TodoList = {
		...before,
		items: [
			{
				...before.items[0],
				id: "read",
				text: "Read more",
				description: "Chapter two",
				completed: true,
				details: null,
			},
		],
	};
	invoke.mockResolvedValueOnce({ status: "ready", data: before });
	await session.loadTodos(before.date);
	const pending = deferred<CommandResponse<TodoList>>();
	invoke
		.mockReturnValueOnce(pending.promise)
		.mockResolvedValueOnce({ status: "ready", data: after });
	const edit = session.editTodo("read", "Read more", "Chapter two");
	await session.loadTodos(before.date);
	await session.loadTodos(before.date);
	expect(invoke).toHaveBeenCalledTimes(2);
	pending.resolve({ status: "ready", data: after });
	expect(await edit).toBe(true);
	await Promise.resolve();
	expect(invoke).toHaveBeenCalledTimes(3);
	expect(session.todos.data?.items[0]?.description).toBe("Chapter two");
	expect(session.todos.data?.items[0]?.completed).toBe(true);
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

it("retains a failed edit error after a queued refresh succeeds", async () => {
	const session = createDashboardSession(() => false, "island");
	const data: TodoList = { date: "2026-09-10", items: [], syncError: null };
	invoke.mockResolvedValueOnce({ status: "ready", data });
	await session.loadTodos(data.date);
	const pending = deferred<CommandResponse<TodoList>>();
	invoke.mockReturnValueOnce(pending.promise).mockResolvedValueOnce({ status: "ready", data });
	const write = session.editTodo("read", "Read", "Notes");
	await session.loadTodos();
	pending.resolve({ status: "failed", message: "Could not save Todo" });
	expect(await write).toBe(false);
	await vi.waitFor(() => expect(session.todos.loading).toBe(false));
	expect(session.todos.error).toBe("Could not save Todo");
	invoke.mockResolvedValueOnce({ status: "ready", data });
	await session.loadTodos();
	expect(session.todos.error).toBe("Could not save Todo");
	invoke.mockResolvedValueOnce({ status: "ready", data: { ...data, date: "2026-09-11" } });
	await session.loadTodos("2026-09-11");
	expect(session.todos.error).toBeNull();
});

it("clears disabled provider failures independently when Rust emits an absent-widget projection", () => {
	const session = createDashboardSession(() => false, "island");
	cleanup = mounts[0]!();
	const providers = {
		codex: session.dashboard.codex,
		openCode: session.dashboard.openCode,
		claude: session.dashboard.claude,
		grok: session.dashboard.grok,
		copilot: session.dashboard.copilot,
		deepSeek: session.dashboard.deepSeek,
		cherryIn: session.dashboard.cherryIn,
		github: session.dashboard.github,
	};
	for (const [source, state] of Object.entries(providers)) {
		listeners.get("dashboard-source-updated")?.({
			payload: { source, result: { status: "failed", message: "Provider unavailable" } },
		});
		expect(state.error).toBe("Provider unavailable");
		listeners.get("dashboard-source-updated")?.({
			payload: { source, result: { status: "ready", data: null } },
		});
		expect(state.error).toBeNull();
		expect(state.data).toBeNull();
		expect(state.loading).toBe(false);
	}
	expect(invoke).not.toHaveBeenCalled();
});
