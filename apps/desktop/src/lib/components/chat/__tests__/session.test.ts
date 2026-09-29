import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";
import { createChatSession } from "../session.svelte";
import type { ChatSnapshot, CommandResponse } from "../../../consumer";

const { invoke, mounts, listeners } = vi.hoisted(() => ({
	invoke: vi.fn(),
	mounts: new Array<() => () => void>(),
	listeners: new Map<string, (event: { payload: ChatSnapshot }) => void>(),
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

function snapshot(revision: number, connected = false): ChatSnapshot {
	return { revision, connected, busy: false, model: "test", messages: [], error: null };
}

function deferred<T>() {
	let resolve: (value: T) => void = () => {
		throw new Error("Promise not initialized");
	};
	const promise = new Promise<T>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}

it("starts offline without connecting and ignores a read older than a streaming event", async () => {
	const read = deferred<CommandResponse<ChatSnapshot>>();
	invoke.mockReturnValue(read.promise);
	const session = createChatSession();
	const mount = mounts[0];
	if (mount === undefined) throw new Error("Expected mount callback");
	cleanup = mount();
	await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("read_chat"));
	expect(session.snapshot.connected).toBe(false);
	expect(invoke).not.toHaveBeenCalledWith("connect_chat");
	listeners.get("chat-updated")?.({ payload: snapshot(3, true) });
	read.resolve({ status: "ready", data: snapshot(1) });
	await Promise.resolve();
	expect(session.snapshot.revision).toBe(3);
	expect(session.snapshot.connected).toBe(true);
});

it("connects without a directory parameter and retains edits made while sending", async () => {
	invoke.mockResolvedValueOnce({ status: "ready", data: snapshot(1, true) });
	const session = createChatSession();
	await session.connect();
	expect(invoke).toHaveBeenCalledWith("connect_chat");
	const sent = deferred<CommandResponse<null>>();
	invoke.mockReturnValueOnce(sent.promise);
	session.draft = "first";
	const pending = session.send();
	session.draft = "next";
	sent.resolve({ status: "ready", data: null });
	await pending;
	expect(session.draft).toBe("next");
	expect(invoke).toHaveBeenCalledWith("send_chat", { message: "first" });
});

it("preserves the draft on failure and rejects updates after unmount", async () => {
	invoke.mockResolvedValue({ status: "ready", data: snapshot(1, true) });
	const session = createChatSession();
	const mount = mounts[0];
	if (mount === undefined) throw new Error("Expected mount callback");
	cleanup = mount();
	await session.connect();
	invoke.mockResolvedValueOnce({ status: "failed", message: "Pi disconnected" });
	session.draft = "keep me";
	await session.send();
	expect(session.draft).toBe("keep me");
	expect(session.error).toBe("Pi disconnected");
	const update = listeners.get("chat-updated");
	cleanup();
	update?.({ payload: snapshot(9) });
	expect(session.snapshot.revision).toBe(1);
});

it("discards the draft and conversation on leaving and ignores late connect replies", async () => {
	const pending = deferred<CommandResponse<ChatSnapshot>>();
	invoke.mockReturnValueOnce(pending.promise);
	const session = createChatSession();
	session.draft = "discard me";
	const connection = session.connect();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: null })
		.mockResolvedValueOnce({ status: "ready", data: snapshot(4) });
	await session.leave();
	pending.resolve({ status: "ready", data: snapshot(2, true) });
	await connection;
	expect(session.draft).toBe("");
	expect(session.snapshot.messages).toEqual([]);
	expect(session.snapshot.connected).toBe(false);
	expect(session.connecting).toBe(false);
	expect(invoke).toHaveBeenCalledWith("control_chat", { action: "disconnect" });
});

it("ignores a control failure from a discarded conversation", async () => {
	const response = deferred<CommandResponse<null>>();
	invoke.mockReturnValueOnce(response.promise);
	const session = createChatSession();
	const control = session.control("stop");
	invoke
		.mockResolvedValueOnce({ status: "ready", data: null })
		.mockResolvedValueOnce({ status: "ready", data: snapshot(4) });
	await session.leave();
	response.resolve({ status: "failed", message: "Old response failed" });
	await control;
	expect(session.error).toBeNull();
	expect(session.snapshot.connected).toBe(false);
});
