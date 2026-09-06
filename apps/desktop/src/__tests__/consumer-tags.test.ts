import { beforeEach, expect, it, vi } from "vite-plus/test";
import type { CommandResponse, MemoTagCount } from "../lib/consumer";
import { createMemosTags } from "../lib/components/memos/session.svelte";
import { createMomentTags } from "../lib/components/moment/session.svelte";

function createConsumerTags() {
	return { memos: createMemosTags(), moment: createMomentTags() };
}

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
beforeEach(() => invoke.mockReset());

function deferred<T>() {
	let settle: (value: T) => void = () => {
		throw new Error("Promise not initialized");
	};
	const promise = new Promise<T>((resolve) => {
		settle = resolve;
	});
	return { promise, resolve: (value: T) => settle(value) };
}

it("shows each index without waiting for the other source and coalesces ordinary refreshes", async () => {
	const moment = deferred<CommandResponse<string[]>>();
	invoke.mockImplementation((command: string) =>
		command === "read_moment_tags"
			? moment.promise
			: Promise.resolve({ status: "ready", data: [{ name: "memo", count: 1 }] }),
	);
	const tags = createConsumerTags();
	const pending = tags.moment.refresh();
	await tags.memos.refresh();
	await tags.moment.refresh();
	expect(tags.memos.tags).toEqual([{ name: "memo", count: 1 }]);
	expect(tags.moment.loading).toBe(true);
	expect(invoke).toHaveBeenCalledTimes(2);
	moment.resolve({ status: "ready", data: ["photo"] });
	await pending;
	expect(tags.moment.tags).toEqual(["photo"]);
});

it("preserves settled tags on failure, exposes errors, retries, and accepts an empty index", async () => {
	invoke
		.mockResolvedValueOnce({ status: "ready", data: ["old"] })
		.mockResolvedValueOnce({ status: "failed", message: "Unavailable" })
		.mockRejectedValueOnce(new Error("IPC unavailable"))
		.mockResolvedValueOnce({ status: "ready", data: [] });
	const { moment } = createConsumerTags();
	await moment.refresh();
	await moment.refresh();
	expect(moment.tags).toEqual(["old"]);
	expect(moment.error).toBe("Unavailable");
	await moment.refresh();
	expect(moment.error).toBe("Could not load tags. Try again.");
	expect(moment.loading).toBe(false);
	await moment.refresh();
	expect(moment.tags).toEqual([]);
	expect(moment.error).toBeNull();
});

it.each(["ready", "failed"])(
	"ignores stale %s responses after mutation refresh",
	async (status) => {
		const old = deferred<CommandResponse<string[]>>();
		const fresh = deferred<CommandResponse<string[]>>();
		invoke.mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise);
		const { moment } = createConsumerTags();
		const first = moment.refresh();
		const second = moment.refresh(true);
		old.resolve(
			status === "ready"
				? { status, data: ["deleted"] }
				: { status: "failed", message: "Old error" },
		);
		await first;
		expect(moment.loading).toBe(true);
		expect(moment.error).toBeNull();
		fresh.resolve({ status: "ready", data: [] });
		await second;
		expect(moment.tags).toEqual([]);
	},
);

it("does not restore an old index when its response arrives after the new one", async () => {
	const old = deferred<CommandResponse<MemoTagCount[]>>();
	invoke.mockReturnValueOnce(old.promise).mockResolvedValueOnce({ status: "ready", data: [] });
	const { memos } = createConsumerTags();
	const first = memos.refresh();
	await memos.refresh(true);
	old.resolve({ status: "ready", data: [{ name: "deleted", count: 1 }] });
	await first;
	expect(memos.tags).toEqual([]);
});

it("clears the previous credential session and rejects its pending responses independently", async () => {
	const old = deferred<CommandResponse<string[]>>();
	invoke
		.mockResolvedValueOnce({ status: "ready", data: ["private"] })
		.mockReturnValueOnce(old.promise)
		.mockResolvedValueOnce({ status: "ready", data: ["new"] });
	const { moment, memos } = createConsumerTags();
	await moment.refresh();
	const first = moment.refresh();
	const previous = moment.session;
	moment.reset();
	expect(moment.session).not.toBe(previous);
	expect(memos.session).toBe(0);
	expect(moment.tags).toEqual([]);
	await moment.refresh();
	old.resolve({ status: "failed", message: "Previous credentials" });
	await first;
	expect(moment.tags).toEqual(["new"]);
	expect(moment.error).toBeNull();
});
