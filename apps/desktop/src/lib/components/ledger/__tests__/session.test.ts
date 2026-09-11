import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";
import type { CommandResponse, ExpenseSnapshot } from "../../../consumer";
import { createLedgerSession } from "../session.svelte";

const { invoke, mounts, listeners } = vi.hoisted(() => ({
	invoke: vi.fn(),
	mounts: [] as Array<() => () => void>,
	listeners: new Map<string, (event: { payload: string }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn(async (name, callback) => {
		listeners.set(name, callback);
		return () => listeners.delete(name);
	}),
}));
vi.mock("svelte", async (original) => ({
	...(await original<typeof import("svelte")>()),
	onMount: (callback: () => () => void) => mounts.push(callback),
}));
let cleanup: (() => void) | null = null;
beforeEach(() => {
	invoke.mockReset();
	mounts.length = 0;
	listeners.clear();
});
afterEach(() => {
	cleanup?.();
	cleanup = null;
});

function snapshot(date: string): ExpenseSnapshot {
	return {
		date,
		month: date.slice(0, 7),
		entries: [],
		dayTotalPence: 0,
		monthTotalPence: 0,
		categories: [],
		days: [],
		suggestions: [],
	};
}
function pending() {
	let resolve: (value: CommandResponse<ExpenseSnapshot>) => void = () => {
		throw new Error("Not initialized");
	};
	const promise = new Promise<CommandResponse<ExpenseSnapshot>>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}

it("rejects an older month response after calendar navigation", async () => {
	const session = createLedgerSession();
	const old = pending();
	invoke
		.mockReturnValueOnce(old.promise)
		.mockResolvedValueOnce({ status: "ready", data: snapshot("2026-10-01") });
	const first = session.load("2026-09-30");
	await session.load("2026-10-01");
	old.resolve({ status: "ready", data: snapshot("2026-09-30") });
	await first;
	expect(session.data?.date).toBe("2026-10-01");
});

it("keeps the submitted expense date and rereads the new selection after a write", async () => {
	const session = createLedgerSession();
	invoke.mockResolvedValueOnce({ status: "ready", data: snapshot("2026-09-30") });
	await session.load("2026-09-30");
	const old = pending();
	invoke
		.mockReturnValueOnce(old.promise)
		.mockResolvedValueOnce({ status: "ready", data: snapshot("2026-10-01") });
	const save = session.save(null, "12.34", "Dining");
	expect(await session.save(null, "12.34", "Dining")).toBe(false);
	await session.load("2026-10-01");
	expect(session.data).toBeNull();
	expect(invoke).toHaveBeenLastCalledWith("create_expense", {
		date: "2026-09-30",
		amount: "12.34",
		category: "Dining",
	});
	old.resolve({ status: "ready", data: snapshot("2026-09-30") });
	expect(await save).toBe(false);
	await vi.waitFor(() => expect(session.data?.date).toBe("2026-10-01"));
});

it("refreshes monthly totals for another day's mutation and preserves write errors", async () => {
	const session = createLedgerSession();
	cleanup = mounts[0]!();
	await Promise.resolve();
	const data = snapshot("2026-09-11");
	invoke.mockResolvedValueOnce({ status: "ready", data });
	await session.load(data.date);
	const old = pending();
	invoke.mockReturnValueOnce(old.promise).mockResolvedValueOnce({ status: "ready", data });
	const save = session.save("entry", "1.20", "Dining");
	listeners.get("expenses-updated")?.({ payload: "2026-09-20" });
	old.resolve({ status: "failed", message: "Could not save expense" });
	expect(await save).toBe(false);
	await vi.waitFor(() => expect(session.loading).toBe(false));
	expect(session.error).toBe("Could not save expense");
	const calls = invoke.mock.calls.length;
	listeners.get("expenses-updated")?.({ payload: "2026-10-01" });
	expect(invoke).toHaveBeenCalledTimes(calls);
	invoke.mockResolvedValueOnce({ status: "ready", data: { ...data, monthTotalPence: 300 } });
	listeners.get("expenses-updated")?.({ payload: "2026-09-20" });
	await vi.waitFor(() => expect(session.data?.monthTotalPence).toBe(300));
	expect(session.error).toBe("Could not save expense");
});

it("discards late responses and removes focus reads after disposal", async () => {
	const session = createLedgerSession();
	cleanup = mounts[0]!();
	await Promise.resolve();
	const old = pending();
	invoke.mockReturnValueOnce(old.promise);
	const read = session.load("2026-09-11");
	cleanup();
	cleanup = null;
	old.resolve({ status: "ready", data: snapshot("2026-09-11") });
	await read;
	window.dispatchEvent(new Event("focus"));
	expect(session.data).toBeNull();
	expect(invoke).toHaveBeenCalledTimes(1);
});
