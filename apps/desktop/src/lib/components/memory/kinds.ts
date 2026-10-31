import type { MemoryNodeKind } from "@/lib/contracts/nowledge";

export const memoryKindOrder: MemoryNodeKind[] = ["memory", "entity", "thread", "skill"];

export const memoryKinds: Record<MemoryNodeKind, { label: string; token: string }> = {
	memory: { label: "Memories", token: "--color-chart-1" },
	entity: { label: "Entities", token: "--color-chart-3" },
	thread: { label: "Threads", token: "--color-chart-5" },
	skill: { label: "Skills", token: "--color-chart-6" },
};
