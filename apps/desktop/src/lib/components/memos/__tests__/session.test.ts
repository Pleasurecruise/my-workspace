import { beforeEach, expect, it, vi } from "vite-plus/test";
import type { ChannelView, CommandResponse, MemoView } from "../../../consumer";
import { createMemosSession } from "../session.svelte";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("svelte", async (importOriginal) => ({
	...(await importOriginal<typeof import("svelte")>()),
	onMount: vi.fn(),
}));
beforeEach(() => invoke.mockReset());
function deferred<T>() {
	let resolve: (value: T) => void = () => {
		throw new Error("Promise not initialized");
	};
	const promise = new Promise<T>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}
const original: MemoView = {
	id: "memo",
	r2Key: "memo.md",
	content: "Original",
	html: "Original",
	tags: [],
	createdAt: "2026-09-10",
	updatedAt: "2026-09-10",
	visibility: "private",
	pinned: false,
	favorite: false,
	archived: false,
	metadataComplete: true,
};
function page(memos: MemoView[]): CommandResponse<ChannelView> {
	return { status: "ready", data: { channel: "memos", memos, nextCursor: null } };
}
it.each(["create", "import", "update", "delete"])(
	"preserves %s after a pre-write refresh settles",
	async (operation) => {
		const session = createMemosSession({ active: false, mainElement: null });
		session.initialize(page([original]), session.version);
		const pending = deferred<CommandResponse<ChannelView>>();
		const saved = {
			...original,
			id: operation === "create" || operation === "import" ? "created" : original.id,
			content: "Saved",
		};
		invoke.mockImplementation((command: string) => {
			if (command === "read_channel") return pending.promise;
			if (command === "read_memo_tags") return Promise.resolve({ status: "ready", data: [] });
			return Promise.resolve({
				status: "ready",
				data: operation === "delete" ? original.id : saved,
			});
		});
		const read = session.refresh();
		if (operation === "create") await session.createMemo("Saved", "private");
		else if (operation === "import")
			await session.importXMemo("https://x.com/example/status/123", "private");
		else if (operation === "update")
			await session.updateMemo(original.id, { content: "Saved", visibility: "private" });
		else await session.deleteMemo(original.id);
		const expected =
			operation === "delete" ? [] : operation === "update" ? [saved] : [saved, original];
		expect(session.content?.memos).toEqual(expected);
		pending.resolve(page([original]));
		await read;
		expect(session.content?.memos).toEqual(expected);
		expect(session.loading).toBe(false);
	},
);
it("allows a pending read to settle after a failed write", async () => {
	const session = createMemosSession({ active: false, mainElement: null });
	session.initialize(page([original]), session.version);
	const pending = deferred<CommandResponse<ChannelView>>();
	invoke.mockImplementation((command: string) =>
		command === "read_channel"
			? pending.promise
			: Promise.resolve({ status: "failed", message: "offline" }),
	);
	const read = session.refresh();
	await session.deleteMemo(original.id);
	pending.resolve(page([{ ...original, content: "Refreshed" }]));
	await read;
	expect(session.content?.memos[0]?.content).toBe("Refreshed");
});

it("restarts a pending filter after a write without mixing its old cursor", async () => {
	const session = createMemosSession({ active: true, mainElement: null });
	session.initialize(
		{
			status: "ready",
			data: { channel: "memos", memos: [original], nextCursor: "old-query-cursor" },
		},
		session.version,
	);
	const old = deferred<CommandResponse<ChannelView>>();
	const current = deferred<CommandResponse<ChannelView>>();
	let reads = 0;
	invoke.mockImplementation((command: string) => {
		if (command === "read_channel") return ++reads === 1 ? old.promise : current.promise;
		if (command === "read_memo_tags") return Promise.resolve({ status: "ready", data: [] });
		return Promise.resolve({ status: "ready", data: { ...original, content: "Saved" } });
	});
	const filter = session.filterMemos("new search", ["new-tag"], true, "active");
	await session.updateMemo(original.id, { content: "Saved", visibility: "private" });
	expect(reads).toBe(2);
	expect(invoke).toHaveBeenLastCalledWith("read_channel", {
		query: {
			channel: "memos",
			cursor: null,
			search: "new search",
			tags: ["new-tag"],
			sortByUpdated: true,
			archivedOnly: false,
			favoritesOnly: false,
		},
	});
	session.loadMore();
	expect(reads).toBe(2);
	old.resolve(page([original]));
	await filter;
	expect(session.loading).toBe(true);
	current.resolve(page([]));
	await vi.waitFor(() => expect(session.content?.memos).toEqual([]));
	expect(session.loading).toBe(false);
});
it("revalidates a settled search after creating a nonmatching memo", async () => {
	const session = createMemosSession({ active: true, mainElement: null });
	invoke.mockResolvedValue(page([]));
	await session.filterMemos("search", [], false, "active");
	const current = deferred<CommandResponse<ChannelView>>();
	invoke.mockImplementation((command: string) => {
		if (command === "read_channel") return current.promise;
		if (command === "read_memo_tags") return Promise.resolve({ status: "ready", data: [] });
		return Promise.resolve({ status: "ready", data: original });
	});
	await session.createMemo(original.content, "private");
	expect(session.loading).toBe(true);
	current.resolve(page([]));
	await vi.waitFor(() => expect(session.content?.memos).toEqual([]));
});

it.each(["create", "import"])(
	"does not duplicate a %s result already observed by a refresh",
	async (operation) => {
		const session = createMemosSession({ active: false, mainElement: null });
		session.initialize(page([]), session.version);
		const pending = deferred<CommandResponse<MemoView>>();
		invoke.mockImplementation((command: string) => {
			if (command === "read_channel") return Promise.resolve(page([original]));
			if (command === "read_memo_tags") return Promise.resolve({ status: "ready", data: [] });
			return pending.promise;
		});
		const write =
			operation === "create"
				? session.createMemo("Original", "private")
				: session.importXMemo("https://x.com/example/status/123", "private");
		await session.refresh();
		pending.resolve({ status: "ready", data: original });
		await write;
		expect(session.content?.memos).toEqual([original]);
	},
);
