import { beforeEach, expect, it, vi } from "vite-plus/test";
import type { CommandResponse, WidgetLayout } from "../../../consumer";
import { createLayoutSession } from "../layout.svelte";

const { invoke, mounts } = vi.hoisted(() => ({
	invoke: vi.fn(),
	mounts: [] as Array<() => () => void>,
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("svelte", async (original) => ({
	...(await original<typeof import("svelte")>()),
	onMount: (callback: () => () => void) => mounts.push(callback),
}));
beforeEach(() => {
	invoke.mockReset();
	mounts.length = 0;
});
const original: WidgetLayout = {
	widgets: [{ id: "todo", widget: { kind: "todoList" } }],
	islandWidgetId: "todo",
};
const unpinned: WidgetLayout = { ...original, islandWidgetId: null };

it("does not replace a newer layout read with a delayed save response", async () => {
	const session = createLayoutSession();
	let finish!: (response: CommandResponse<null>) => void;
	invoke.mockReturnValueOnce(
		new Promise<CommandResponse<null>>((resolve) => {
			finish = resolve;
		}),
	);
	const save = session.save(original);
	invoke.mockResolvedValueOnce({ status: "ready", data: unpinned });
	await session.load();
	finish({ status: "ready", data: null });
	expect(await save).toBe(true);
	expect(session.layout?.islandWidgetId).toBeNull();
	expect(session.saving).toBe(false);
});

it("ignores startup responses and subsequent reads after disposal", async () => {
	let finish!: (response: CommandResponse<WidgetLayout>) => void;
	const pending = new Promise<CommandResponse<WidgetLayout>>((resolve) => {
		finish = resolve;
	});
	invoke.mockImplementation((command) => {
		if (command === "read_layout") return pending;
		if (command === "island_available") return Promise.resolve(true);
		return Promise.resolve({ status: "ready", data: [] });
	});
	const session = createLayoutSession();
	mounts[0]!()();
	finish({ status: "ready", data: original });
	await pending;
	await session.load();
	expect(session.layout).toBeNull();
	expect(session.islandAvailable).toBe(false);
	expect(invoke.mock.calls.filter(([command]) => command === "read_layout")).toHaveLength(1);
});
