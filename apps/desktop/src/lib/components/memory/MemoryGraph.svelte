<script lang="ts">
	import { forceCollide, forceLink, forceManyBody, forceSimulation, forceX, forceY, type Simulation, type SimulationNodeDatum } from "d3-force";
	import { select } from "d3-selection";
	import { zoom, zoomIdentity } from "d3-zoom";
	import type { MemoryGraph, MemoryNodeKind } from "@/lib/contracts/nowledge";
	import { memoryKinds } from "./kinds";

	interface Point extends SimulationNodeDatum {
		id: string;
		kind: MemoryNodeKind;
		title: string;
		degree: number;
		radius: number;
		x: number;
		y: number;
	}

	let { graph, visible, selected, focus, onselect }: {
		graph: MemoryGraph;
		visible: Record<MemoryNodeKind, boolean>;
		selected: string | null;
		focus: Set<string> | null;
		onselect: (id: string | null) => void;
	} = $props();

	let width = $state(0);
	let height = $state(0);
	let svg = $state<SVGSVGElement | null>(null);
	let transform = $state.raw(zoomIdentity);
	let layout = $state.raw<{ nodes: Point[]; edges: Array<{ key: string; relation: string; source: Point; target: Point }> }>({ nodes: [], edges: [] });
	let simulation: Simulation<Point, undefined> | null = null;
	let drag: { point: Point; x: number; y: number; moved: boolean } | null = null;

	$effect(() => {
		const points: Point[] = graph.nodes.map((node, index) => ({
			id: node.id,
			kind: node.kind,
			title: node.title,
			degree: node.degree,
			radius: 4 + Math.sqrt(node.degree) * 2,
			x: 12 * Math.sqrt(index + 0.5) * Math.cos(index * 2.4),
			y: 12 * Math.sqrt(index + 0.5) * Math.sin(index * 2.4),
		}));
		const byId = new Map(points.map((point) => [point.id, point]));
		const pairs = graph.edges.flatMap((edge, index) => {
			const source = byId.get(edge.source);
			const target = byId.get(edge.target);
			return source === undefined || target === undefined ? [] : [{ key: `${index}`, relation: edge.relation, source, target }];
		});
		const publish = () => {
			layout = {
				nodes: points.map((point) => ({ ...point })),
				edges: pairs.map((pair) => ({ ...pair, source: { ...pair.source }, target: { ...pair.target } })),
			};
		};
		const created = forceSimulation(points)
			.force("link", forceLink<Point, { source: Point; target: Point }>(pairs.map(({ source, target }) => ({ source, target }))).distance(48))
			.force("charge", forceManyBody().strength(-90))
			.force("collide", forceCollide<Point>((point) => point.radius + 3))
			.force("x", forceX(0).strength(0.05))
			.force("y", forceY(0).strength(0.05))
			.on("tick", publish);
		if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
			created.stop();
			created.tick(300);
			publish();
		}
		simulation = created;
		return () => {
			created.stop();
			simulation = null;
		};
	});

	function zoomable(element: SVGSVGElement) {
		const behavior = zoom<SVGSVGElement, unknown>()
			.scaleExtent([0.25, 4])
			.filter((event: Event) => !(event.target instanceof Element && event.target.closest(".node") !== null) && !(event instanceof MouseEvent && event.button !== 0))
			.on("zoom", (event: { transform: typeof zoomIdentity }) => {
				transform = event.transform;
			});
		const selection = select(element).call(behavior);
		return () => {
			selection.on(".zoom", null);
		};
	}

	function grab(event: PointerEvent & { currentTarget: SVGCircleElement }, id: string) {
		const point = layout.nodes.find((candidate) => candidate.id === id);
		const live = simulation === null ? undefined : simulation.nodes().find((candidate) => candidate.id === id);
		if (event.button !== 0 || point === undefined || live === undefined) return;
		event.currentTarget.setPointerCapture(event.pointerId);
		drag = { point: live, x: event.clientX, y: event.clientY, moved: false };
	}

	function move(event: PointerEvent) {
		if (drag === null || svg === null || simulation === null) return;
		if (!drag.moved && Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 3) return;
		drag.moved = true;
		const bounds = svg.getBoundingClientRect();
		const [x, y] = transform.invert([event.clientX - bounds.left - width / 2, event.clientY - bounds.top - height / 2]);
		drag.point.fx = x;
		drag.point.fy = y;
		simulation.alphaTarget(0.3).restart();
	}

	function release() {
		if (drag === null) return;
		if (!drag.moved) onselect(drag.point.id);
		drag.point.fx = null;
		drag.point.fy = null;
		if (simulation !== null) simulation.alphaTarget(0);
		drag = null;
	}
</script>

<div class="canvas" bind:clientWidth={width} bind:clientHeight={height}>
	<svg bind:this={svg} {@attach zoomable} viewBox={`${-width / 2} ${-height / 2} ${width} ${height}`} aria-label="Memory graph">
		<g transform={transform.toString()}>
			{#each layout.edges as edge (edge.key)}
				{#if visible[edge.source.kind] && visible[edge.target.kind]}
					<line
						class="edge"
						class:dimmed={focus !== null && edge.source.id !== selected && edge.target.id !== selected}
						x1={edge.source.x}
						y1={edge.source.y}
						x2={edge.target.x}
						y2={edge.target.y}
					><title>{edge.relation}</title></line>
				{/if}
			{/each}
			{#each layout.nodes as point (point.id)}
				{#if visible[point.kind]}
					{@const dimmed = focus !== null && !focus.has(point.id)}
					<circle
						class="node"
						class:dimmed
						class:selected={point.id === selected}
						style:--kind-color={`var(${memoryKinds[point.kind].token})`}
						cx={point.x}
						cy={point.y}
						r={point.radius}
						role="button"
						tabindex="0"
						aria-label={point.title}
						aria-pressed={point.id === selected}
						onpointerdown={(event) => grab(event, point.id)}
						onpointermove={move}
						onpointerup={release}
						onpointercancel={release}
						onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onselect(point.id); } }}
					/>
					{#if (focus !== null && focus.has(point.id)) || (focus === null && (point.degree >= 5 || transform.k >= 1.6))}
						<text class="label" x={point.x} y={point.y + point.radius + 11}>{point.title.length > 28 ? `${point.title.slice(0, 27)}…` : point.title}</text>
					{/if}
				{/if}
			{/each}
		</g>
	</svg>
</div>

<style>
	.canvas { position: relative; flex: 1; min-width: 0; min-height: 0; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: color-mix(in srgb, var(--color-muted) 35%, var(--color-background)); }
	svg { display: block; width: 100%; height: 100%; cursor: grab; touch-action: none; }
	svg:active { cursor: grabbing; }
	.edge { stroke: var(--color-border-strong); stroke-width: 1; transition: opacity var(--duration-base); }
	.node { stroke: var(--color-background); stroke-width: 1.5; cursor: pointer; transition: opacity var(--duration-base); }
	.node:focus-visible { outline: none; stroke: var(--color-foreground); stroke-width: 2.5; }
	.node.selected { stroke: var(--color-foreground); stroke-width: 2.5; }
	.node { fill: var(--kind-color); }
	.dimmed { opacity: 0.15; }
	.label { fill: var(--color-foreground); font-family: var(--font-sans); font-size: 10px; text-anchor: middle; paint-order: stroke; stroke: var(--color-background); stroke-width: 3px; pointer-events: none; }
	@media (prefers-reduced-motion: reduce) {
		.edge, .node { transition: none; }
	}
</style>
