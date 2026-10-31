import {
	forceLink,
	forceManyBody,
	forceSimulation,
	forceX,
	forceY,
	type SimulationNodeDatum,
} from "d3-force";
import type { MemoryGraph, MemoryNode, MemoryNodeKind } from "@/lib/contracts/nowledge";

export type HeightMode = "influence" | "structure" | "forms" | "growth";

const recency = (days: number) => 1 - Math.min(1, Math.log1p(Math.max(0, days)) / Math.log1p(400));
const forms: Record<MemoryNodeKind, number> = { thread: 0, memory: 1 / 3, entity: 2 / 3, skill: 1 };

export const heightModes: Array<{
	id: HeightMode;
	label: string;
	title: string;
	caption: string;
	ticks: Array<{ text: string; value: number }>;
}> = [
	{
		id: "influence",
		label: "Influence",
		title: "Influence",
		caption: "Peaks are memories that matter most and connect widely.",
		ticks: [],
	},
	{
		id: "structure",
		label: "Structure",
		title: "Structural depth",
		caption: "Peaks belong to the densest core; loosely attached nodes settle low.",
		ticks: [
			{ text: "Backbone", value: 1 },
			{ text: "Inner network", value: 0.75 },
			{ text: "Connected mesh", value: 0.5 },
			{ text: "Outer ring", value: 0.25 },
			{ text: "Isolated", value: 0 },
		],
	},
	{
		id: "forms",
		label: "Forms",
		title: "Knowledge form",
		caption: "Conversation traces sit at the base and reusable skills at the top.",
		ticks: [
			{ text: "Skills", value: 1 },
			{ text: "Entities", value: 2 / 3 },
			{ text: "Memory units", value: 1 / 3 },
			{ text: "Traces", value: 0 },
		],
	},
	{
		id: "growth",
		label: "Growth",
		title: "Recorded growth",
		caption:
			"Recent records rise above older ones; entities follow the memories that mention them.",
		ticks: [
			{ text: "Now", value: 1 },
			{ text: "7 days", value: recency(7) },
			{ text: "30 days", value: recency(30) },
			{ text: "1+ years", value: recency(365) },
		],
	},
];

export function elevation(
	nodes: MemoryNode[],
	mode: HeightMode,
	now: number,
): (node: MemoryNode) => number | null {
	const maxCore = Math.max(0, ...nodes.map((node) => node.core));
	return (node) => {
		if (mode === "influence") return node.influence ** 2.5;
		if (mode === "structure") return maxCore === 0 ? 0 : node.core / maxCore;
		if (mode === "forms") return forms[node.kind];
		return node.recordedAt === null
			? null
			: recency((now - Date.parse(node.recordedAt)) / 86_400_000);
	};
}

export function groundLayout(graph: MemoryGraph): Map<string, { x: number; z: number }> {
	const datums = new Map(
		graph.nodes.map((node, slot): [string, SimulationNodeDatum & { x: number; y: number }] => [
			node.id,
			{
				x: 8 * Math.sqrt(slot + 0.5) * Math.cos(slot * 2.4),
				y: 8 * Math.sqrt(slot + 0.5) * Math.sin(slot * 2.4),
			},
		]),
	);
	const links = graph.edges.flatMap((edge) => {
		const source = datums.get(edge.source);
		const target = datums.get(edge.target);
		return source === undefined || target === undefined ? [] : [{ source, target }];
	});
	forceSimulation([...datums.values()])
		.force("link", forceLink(links).distance(10).strength(0.7))
		.force("charge", forceManyBody().strength(-14))
		.force("x", forceX(0).strength(0.05))
		.force("y", forceY(0).strength(0.05))
		.stop()
		.tick(260);
	const extent = Math.max(1, ...[...datums.values()].map((datum) => Math.hypot(datum.x, datum.y)));
	return new Map(
		[...datums].map(([id, datum]) => [id, { x: datum.x / extent, z: datum.y / extent }]),
	);
}
