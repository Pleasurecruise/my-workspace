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
	vi.useRealTimers();
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
	session.selectDate("2026-09-07");
	const older = session.loadTodos();
	session.selectDate("2026-09-08");
	await session.loadTodos();
	resolve({ status: "ready", data: { date: "2026-09-07", items: [], syncError: null } });
	await older;
	expect(session.selectedDate).toBe("2026-09-08");
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
	session.selectDate("2026-09-07");
	const read = session.loadTodos();
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
	session.selectDate("2026-09-07");
	const read = session.loadTodos();
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
	session.selectDate("2026-09-10");
	await session.loadTodos();
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
	session.selectDate("2026-09-11");
	await session.loadTodos();
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

it("retains an explicit Notion refresh requested during an edit", async () => {
	const session = createDashboardSession(() => false, "island");
	const before: TodoList = {
		date: "2026-09-10",
		syncError: null,
		items: [
			{
				id: "read",
				text: "Read",
				description: null,
				completed: true,
				rollover: false,
				details: null,
			},
		],
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
				rollover: false,
				details: null,
			},
		],
	};
	invoke.mockResolvedValueOnce({ status: "ready", data: before });
	session.selectDate(before.date);
	await session.loadTodos();
	const pending = deferred<CommandResponse<TodoList>>();
	invoke
		.mockReturnValueOnce(pending.promise)
		.mockResolvedValueOnce({ status: "ready", data: after });
	const edit = session.editTodo("read", "Read more", "Chapter two");
	session.selectDate(before.date);
	await session.loadTodos(true);
	session.selectDate(before.date);
	await session.loadTodos();
	expect(invoke).toHaveBeenCalledTimes(2);
	pending.resolve({ status: "ready", data: after });
	expect(await edit).toBe(true);
	await Promise.resolve();
	expect(invoke).toHaveBeenCalledTimes(3);
	expect(invoke).toHaveBeenLastCalledWith("read_todos", { date: before.date, refresh: true });
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
	session.selectDate(data.date);
	await session.loadTodos();
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
	session.selectDate("2026-09-11");
	await session.loadTodos();
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
		tokenFlux: session.dashboard.tokenFlux,
		dimAgent: session.dashboard.dimAgent,
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

it("advances the Planner date even when the new day's task read fails", async () => {
	const session = createDashboardSession(() => false, "island");
	cleanup = mounts[0]!();
	invoke.mockResolvedValue({ status: "failed", message: "Invalid ICS file" });
	listeners.get("planner-date-changed")?.({ payload: "2026-09-12" });
	await vi.waitFor(() => expect(session.todos.loading).toBe(false));
	expect(session.todayDate).toBe("2026-09-12");
	expect(session.selectedDate).toBe("2026-09-12");
	expect(session.todos.error).toBe("Invalid ICS file");
	expect(invoke).toHaveBeenCalledWith("read_todos", { date: "2026-09-12" });
});

it("keeps an explicitly selected history date when the local day changes", async () => {
	const session = createDashboardSession(() => false, "island");
	cleanup = mounts[0]!();
	invoke.mockResolvedValue({
		status: "ready",
		data: { date: "2024-02-29", items: [], syncError: null },
	});
	session.selectDate("2024-02-29");
	await session.loadTodos();
	listeners.get("planner-date-changed")?.({ payload: "2026-09-12" });
	await vi.waitFor(() => expect(session.todos.loading).toBe(false));
	expect(session.todayDate).toBe("2026-09-12");
	expect(session.selectedDate).toBe("2024-02-29");
	expect(invoke).toHaveBeenLastCalledWith("read_todos", { date: "2024-02-29" });
});

it("reads the local date on refresh and defers rollover tasks until a pending write finishes", async () => {
	vi.useFakeTimers({ toFake: ["Date"] });
	vi.setSystemTime(new Date("2026-09-11T12:00:00Z"));
	const session = createDashboardSession(() => false, "island");
	cleanup = mounts[0]!();
	const initial = { date: session.todayDate, items: [], syncError: null };
	invoke.mockResolvedValueOnce({ status: "ready", data: initial });
	await session.loadTodos();
	const pending = deferred<CommandResponse<TodoList>>();
	invoke.mockReturnValueOnce(pending.promise);
	const write = session.addTodo("Read", "");
	listeners.get("planner-date-changed")?.({ payload: "2026-09-12" });
	expect(session.selectedDate).toBe("2026-09-12");
	expect(invoke).toHaveBeenCalledTimes(2);
	invoke.mockResolvedValueOnce({ status: "ready", data: { ...initial, date: "2026-09-12" } });
	pending.resolve({ status: "ready", data: initial });
	expect(await write).toBe(false);
	await vi.waitFor(() => expect(session.todos.data?.date).toBe("2026-09-12"));
	invoke
		.mockResolvedValueOnce({ status: "ready", data: "2026-09-13" })
		.mockResolvedValueOnce({ status: "ready", data: { ...initial, date: "2026-09-13" } });
	await session.refreshPlanner();
	expect(session.todayDate).toBe("2026-09-13");
	expect(session.selectedDate).toBe("2026-09-13");
	expect(session.todos.data?.date).toBe("2026-09-13");
});

it("saves Todo order, retains settled tasks on failure, and ignores a save after changing dates", async () => {
	const session = createDashboardSession(() => false, "island");
	const data: TodoList = {
		date: "2026-09-12",
		syncError: null,
		items: ["First", "Second"].map((text) => ({
			id: text,
			text,
			completed: false,
			rollover: false,
			description: null,
			details: null,
		})),
	};
	session.selectDate(data.date);
	invoke.mockResolvedValueOnce({ status: "ready", data });
	await session.loadTodos();
	const reordered = { ...data, items: [...data.items].reverse() };
	invoke.mockResolvedValueOnce({ status: "ready", data: reordered });
	await session.reorderTodos(["Second", "First"]);
	expect(invoke).toHaveBeenLastCalledWith("reorder_todos", {
		date: data.date,
		ids: ["Second", "First"],
	});
	expect(session.todos.data).toEqual(reordered);
	invoke.mockResolvedValueOnce({ status: "failed", message: "List changed" });
	await session.reorderTodos(["First", "Second"]);
	expect(session.todos.data).toEqual(reordered);
	expect(session.todos.error).toBe("List changed");
	const pending = deferred<CommandResponse<TodoList>>();
	invoke.mockReturnValueOnce(pending.promise);
	const write = session.reorderTodos(["First", "Second"]);
	session.selectDate("2026-09-13");
	invoke.mockResolvedValueOnce({
		status: "ready",
		data: { ...data, date: "2026-09-13", items: [] },
	});
	await session.loadTodos();
	pending.resolve({ status: "ready", data });
	await write;
	await vi.waitFor(() => expect(session.todos.data?.date).toBe("2026-09-13"));
	expect(session.todos.data?.items).toEqual([]);
});

it("persists carry-forward preference and retains its saved value after failure", async () => {
	const session = createDashboardSession(() => false, "island");
	const data: TodoList = {
		date: "2026-09-12",
		syncError: null,
		items: [
			{
				id: "read",
				text: "Read",
				description: null,
				completed: false,
				rollover: false,
				details: null,
			},
		],
	};
	session.selectDate(data.date);
	invoke.mockResolvedValueOnce({ status: "ready", data });
	await session.loadTodos();
	const saved = { ...data, items: data.items.map((item) => ({ ...item, rollover: true })) };
	invoke.mockResolvedValueOnce({ status: "ready", data: saved });
	await session.setTodoRollover("read", true);
	expect(invoke).toHaveBeenLastCalledWith("set_todo_rollover", {
		date: data.date,
		id: "read",
		rollover: true,
	});
	expect(session.todos.data?.items[0]?.rollover).toBe(true);
	invoke.mockResolvedValueOnce({ status: "failed", message: "Could not save" });
	await session.setTodoRollover("read", false);
	expect(session.todos.data?.items[0]?.rollover).toBe(true);
	expect(session.todos.error).toBe("Could not save");
});

it("releases carry-forward and reorder controls after a rejected transport call", async () => {
	const session = createDashboardSession(() => false, "island");
	session.selectDate("2026-09-12");
	invoke.mockRejectedValueOnce(new Error("Bridge unavailable"));
	await session.setTodoRollover("read", true);
	expect(session.todos.loading).toBe(false);
	expect(session.todos.error).toContain("carry-forward");
	invoke.mockRejectedValueOnce(new Error("Bridge unavailable"));
	expect(await session.reorderTodos(["read"])).toBe(false);
	expect(session.todos.loading).toBe(false);
	expect(session.todos.error).toContain("Todo order");
});
