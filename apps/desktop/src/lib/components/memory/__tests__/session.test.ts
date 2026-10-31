import { beforeEach, expect, it, vi } from "vite-plus/test";
import { createMemorySession } from "../session.svelte";
import type { CommandResponse } from "@/lib/contracts/command";
import type { MemoryGraph } from "@/lib/contracts/nowledge";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
beforeEach(() => invoke.mockReset());

function graph(title: string): MemoryGraph {
	return {
		nodes: [
			{
				id: title,
				kind: "memory",
				title,
				summary: "",
				category: null,
				recordedAt: null,
				degree: 0,
				core: 0,
				influence: 0.5,
				source: null,
				updatedAt: null,
			},
		],
		edges: [],
		communities: [],
	};
}

it("ignores a slower earlier response", async () => {
	const pending = (): ((response: CommandResponse<MemoryGraph>) => void) => () => {
		throw new Error("Promise not initialized");
	};
	let resolveOlder = pending();
	let resolveNewer = pending();
	invoke
		.mockImplementationOnce(() => new Promise((resolve) => (resolveOlder = resolve)))
		.mockImplementationOnce(() => new Promise((resolve) => (resolveNewer = resolve)));
	const session = createMemorySession();
	const older = session.refresh();
	const newer = session.refresh();
	resolveNewer({ status: "ready", data: graph("newer") });
	await newer;
	resolveOlder({ status: "ready", data: graph("older") });
	await older;
	expect(session.graph).toEqual(graph("newer"));
	expect(session.loading).toBe(false);
});

it("keeps the settled graph when a refresh fails", async () => {
	invoke.mockResolvedValueOnce({ status: "ready", data: graph("settled") });
	invoke.mockResolvedValueOnce({ status: "failed", message: "Nowledge Mem request failed" });
	const session = createMemorySession();
	await session.refresh();
	await session.refresh();
	expect(session.graph).toEqual(graph("settled"));
	expect(session.error).toBe("Nowledge Mem request failed");
});
