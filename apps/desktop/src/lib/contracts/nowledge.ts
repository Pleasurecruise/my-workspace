export type MemoryNodeKind = "memory" | "entity" | "thread" | "skill";

export interface MemoryNode {
	id: string;
	kind: MemoryNodeKind;
	title: string;
	summary: string;
	category: string | null;
	source: string | null;
	updatedAt: string | null;
	recordedAt: string | null;
	degree: number;
	core: number;
	influence: number;
}

export interface MemoryEdge {
	source: string;
	target: string;
	relation: string;
	reason: string | null;
}

export interface MemoryCommunity {
	id: string;
	name: string;
	members: string[];
}

export interface MemoryGraph {
	nodes: MemoryNode[];
	edges: MemoryEdge[];
	communities: MemoryCommunity[];
}
