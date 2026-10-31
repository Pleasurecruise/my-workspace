<script lang="ts">
	import { RefreshCw, X } from "@lucide/svelte";
	import { Badge, Button } from "@my-workspace/ui";
	import MemoryGraph from "../memory/MemoryGraph.svelte";
	import { memoryKindOrder, memoryKinds } from "../memory/kinds";
	import type { createMemorySession } from "../memory/session.svelte";
	import type { MemoryNodeKind } from "@/lib/contracts/nowledge";

	let { session }: { session: ReturnType<typeof createMemorySession> } = $props();
	let selected = $state<string | null>(null);
	let dimension = $state<"2d" | "3d">("3d");
	let visible = $state<Record<MemoryNodeKind, boolean>>({ memory: true, entity: true, thread: true, skill: true });

	const dateFormatter = new Intl.DateTimeFormat("en-GB", { dateStyle: "medium" });
	const graph = $derived(session.graph);
	const node = $derived(graph === null || selected === null ? undefined : graph.nodes.find((candidate) => candidate.id === selected));
	const community = $derived(graph === null || node === undefined ? undefined : graph.communities.find((candidate) => candidate.members.includes(node.id)));
	const connections = $derived.by(() => {
		if (graph === null || node === undefined) return [];
		return graph.edges.flatMap((edge) => {
			const other = edge.source === node.id ? edge.target : edge.target === node.id ? edge.source : null;
			const neighbor = other === null ? undefined : graph.nodes.find((candidate) => candidate.id === other);
			return neighbor === undefined ? [] : [{ neighbor, relation: edge.relation.toLowerCase().replaceAll("_", " "), reason: edge.reason }];
		});
	});
	const focus = $derived(node === undefined ? null : new Set([node.id, ...connections.map(({ neighbor }) => neighbor.id)]));
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape") selected = null; }} />

<section class="memory" aria-label="Memory">
	<header class="page-header">
		<div>
			<h1>Memory</h1>
			<p class="page-description">{graph === null ? "Nowledge Mem knowledge graph" : `${graph.nodes.length} nodes · ${graph.edges.length} links`}</p>
		</div>
		<div class="actions">
			<div class="dimensions" role="group" aria-label="View">
				<button type="button" aria-pressed={dimension === "2d"} onclick={() => (dimension = "2d")}>2D</button>
				<button type="button" aria-pressed={dimension === "3d"} onclick={() => (dimension = "3d")}>3D</button>
			</div>
			<Button variant="ghost" size="icon-sm" aria-label="Refresh memory graph" disabled={session.loading} onclick={() => void session.refresh()}><span class:spinning={session.loading}><RefreshCw size={14} /></span></Button>
		</div>
	</header>

	{#if session.error !== null}<p class="error" role="alert">{session.error}</p>{/if}

	{#if graph === null}
		<p class="status" role="status">{session.loading ? "Loading the memory graph…" : "The memory graph is unavailable."}</p>
	{:else}
		<div class="legend" role="group" aria-label="Node kinds">
			{#each memoryKindOrder as kind (kind)}
				<button type="button" aria-pressed={visible[kind]} onclick={() => (visible[kind] = !visible[kind])}>
					<span class="dot" style:background={`var(${memoryKinds[kind].token})`}></span>{memoryKinds[kind].label}<small>{graph.nodes.filter((node) => node.kind === kind).length}</small>
				</button>
			{/each}
		</div>
		<div class="workspace">
			{#if dimension === "3d"}
				{#await import("../memory/MemoryTerrain.svelte") then { default: MemoryTerrain }}
					<MemoryTerrain {graph} {visible} {selected} {focus} onselect={(id) => (selected = id)} />
				{/await}
			{:else}
				<MemoryGraph {graph} {visible} {selected} {focus} onselect={(id) => (selected = id)} />
			{/if}
			{#if node !== undefined}
				<aside class="details" aria-label="Selected node">
					<div class="details-header">
						<div class="kind-line"><Badge variant="outline" size="sm">{node.kind}</Badge>{#if node.category !== null}<span>{node.category.toLowerCase()}</span>{/if}</div>
						<Button variant="ghost" size="icon-sm" aria-label="Close details" onclick={() => (selected = null)}><X size={14} /></Button>
					</div>
					<h2>{node.title}</h2>
					{#if node.summary}<p class="summary">{node.summary}</p>{/if}
					<dl>
						{#if community !== undefined}<dt>Community</dt><dd>{community.name}</dd>{/if}
						{#if node.source !== null}<dt>Source</dt><dd>{node.source}</dd>{/if}
						{#if node.updatedAt !== null}<dt>Updated</dt><dd><time datetime={node.updatedAt}>{dateFormatter.format(new Date(node.updatedAt))}</time></dd>{/if}
					</dl>
					{#if connections.length > 0}
						<h3>Connections</h3>
						<ul>
							{#each connections as connection, index (index)}
								<li>
									<button type="button" onclick={() => (selected = connection.neighbor.id)}>
										<span>{connection.neighbor.title}</span>
										<small>{connection.relation}{connection.reason === null ? "" : ` · ${connection.reason}`}</small>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</aside>
			{/if}
		</div>
	{/if}
</section>

<style>
	.memory { display: flex; flex: 1; flex-direction: column; width: 100%; min-height: 0; }
	.page-header, .legend, .error, .status { flex-shrink: 0; }
	.actions { display: flex; align-items: center; gap: 0.5rem; }
	.dimensions { display: flex; padding: 0.125rem; border: 1px solid var(--color-border); border-radius: var(--radius-full); }
	.dimensions button, .legend button { border: 1px solid transparent; border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font-family: var(--font-sans); font-size: 0.75rem; transition: background-color var(--duration-fast), color var(--duration-fast), opacity var(--duration-fast); }
	.dimensions button { padding: 0.125rem 0.625rem; }
	.dimensions button[aria-pressed="true"] { background: var(--color-muted); color: var(--color-foreground); }
	.legend { display: flex; flex-wrap: wrap; gap: 0.375rem; margin-bottom: 0.75rem; }
	.legend button { display: inline-flex; align-items: center; gap: 0.375rem; padding: 0.25rem 0.625rem; border-color: var(--color-border); color: var(--color-foreground); }
	.legend button[aria-pressed="false"] { opacity: 0.45; }
	.legend small { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.6875rem; }
	.dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; }
	.workspace { display: flex; flex: 1; gap: 1rem; min-height: 0; }
	.details { display: flex; flex-direction: column; gap: 0.75rem; width: 20rem; flex-shrink: 0; min-height: 0; overflow-y: auto; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); scrollbar-width: thin; scrollbar-color: var(--color-border-strong) transparent; }
	.details-header { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
	.kind-line { display: flex; align-items: center; gap: 0.5rem; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.details h2 { margin: 0; font-size: 1rem; }
	.summary { margin: 0; color: var(--color-muted-foreground); font-size: 0.8125rem; line-height: 1.6; white-space: pre-wrap; overflow-wrap: anywhere; }
	dl { display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.75rem; margin: 0; font-size: 0.75rem; }
	dt { color: var(--color-muted-foreground); }
	dd { margin: 0; overflow-wrap: anywhere; }
	.details h3 { margin: 0.25rem 0 0; font-size: 0.8125rem; }
	ul { display: grid; gap: 0.25rem; margin: 0; padding: 0; list-style: none; }
	li button { display: grid; gap: 0.125rem; width: 100%; padding: 0.375rem 0.5rem; border: 0; border-radius: var(--radius-md); background: transparent; color: var(--color-foreground); cursor: pointer; font: inherit; font-size: 0.8125rem; text-align: left; }
	li button:hover, li button:focus-visible { background: var(--color-muted); }
	li small { color: var(--color-muted-foreground); font-size: 0.6875rem; }
	.status { color: var(--color-muted-foreground); font-size: 0.875rem; }
	.error { margin: 0 0 0.75rem; color: var(--color-error); font-size: 0.8125rem; }
	.spinning { display: inline-flex; animation: spin var(--duration-spinner) linear infinite; }
	@keyframes spin { to { rotate: 360deg; } }
	@media (prefers-reduced-motion: reduce) {
		.spinning { animation: none; }
		.dimensions button, .legend button { transition: none; }
	}
	@media (max-width: 760px) {
		.workspace { flex-direction: column; }
		.details { width: auto; max-height: 45%; }
	}
</style>
