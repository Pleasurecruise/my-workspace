import { expect, it } from "vite-plus/test";
import { elevation, groundLayout } from "../terrain";
import type { MemoryNode } from "@/lib/contracts/nowledge";

function node(id: string, overrides: Partial<MemoryNode>): MemoryNode {
	return {
		id,
		kind: "memory",
		title: id,
		summary: "",
		category: null,
		source: null,
		updatedAt: null,
		recordedAt: null,
		degree: 0,
		core: 0,
		influence: 0,
		...overrides,
	};
}

it("maps each height mode to a bounded elevation", () => {
	const core = node("core", {
		kind: "skill",
		core: 4,
		influence: 1,
		recordedAt: "2026-10-31T00:00:00Z",
	});
	const edge = node("edge", { kind: "thread", core: 1, influence: 0.5 });
	const nodes = [core, edge];
	const now = Date.parse("2026-10-31T00:00:00Z");

	expect(elevation(nodes, "influence", now)(core)).toBe(1);
	expect(elevation(nodes, "influence", now)(edge)).toBeLessThan(0.5);
	expect(elevation(nodes, "structure", now)(edge)).toBe(0.25);
	expect(elevation(nodes, "forms", now)(core)).toBe(1);
	expect(elevation(nodes, "forms", now)(edge)).toBe(0);
	expect(elevation(nodes, "growth", now)(core)).toBe(1);
	expect(elevation(nodes, "growth", now)(edge)).toBeNull();
});

it("places every node inside the unit disc", () => {
	const nodes = ["a", "b", "c", "d"].map((id) => node(id, {}));
	const ground = groundLayout({
		nodes,
		edges: [{ source: "a", target: "b", relation: "RELATES_TO", reason: null }],
		communities: [],
	});

	expect([...ground.keys()]).toEqual(["a", "b", "c", "d"]);
	for (const point of ground.values())
		expect(Math.hypot(point.x, point.z)).toBeLessThanOrEqual(1 + 1e-9);
});
