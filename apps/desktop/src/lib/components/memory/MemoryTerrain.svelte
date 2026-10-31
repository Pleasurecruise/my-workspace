<script lang="ts">
	import { BufferAttribute, BufferGeometry, Color, LineBasicMaterial, LineSegments, LinearSRGBColorSpace, PerspectiveCamera, Points, PolarGridHelper, Raycaster, Scene, ShaderMaterial, Sphere, Vector2, Vector3, WebGLRenderer } from "three";
	import { OrbitControls } from "three/addons/controls/OrbitControls.js";
	import { ChevronDown, Mountain } from "@lucide/svelte";
	import type { MemoryGraph, MemoryNode, MemoryNodeKind } from "@/lib/contracts/nowledge";
	import { memoryKinds } from "./kinds";
	import { elevation, groundLayout, heightModes, type HeightMode } from "./terrain";

	interface Spot {
		node: MemoryNode;
		slot: number;
		x: number;
		z: number;
		height: number;
		target: number;
		opacity: number;
		stroke: Color;
	}

	interface Layer {
		spots: Spot[];
		index: Map<string, Spot>;
		positions: BufferAttribute;
		tints: BufferAttribute;
		opacities: BufferAttribute;
	}

	let { graph, visible, selected, focus, onselect }: {
		graph: MemoryGraph;
		visible: Record<MemoryNodeKind, boolean>;
		selected: string | null;
		focus: Set<string> | null;
		onselect: (id: string | null) => void;
	} = $props();

	const RADIUS = 120;
	const HEIGHT = 95;
	const vertexShader = `
		attribute float size;
		attribute vec3 tint;
		attribute float opacity;
		uniform float scale;
		varying vec3 vTint;
		varying float vOpacity;
		void main() {
			vTint = tint;
			vOpacity = opacity;
			vec4 view = modelViewMatrix * vec4(position, 1.0);
			gl_PointSize = size * scale / -view.z;
			gl_Position = projectionMatrix * view;
		}
	`;
	const fragmentShader = `
		varying vec3 vTint;
		varying float vOpacity;
		void main() {
			float radius = length(gl_PointCoord - 0.5);
			if (radius > 0.5 || vOpacity <= 0.0) discard;
			gl_FragColor = vec4(vTint, vOpacity * smoothstep(0.5, 0.38, radius));
		}
	`;

	let mode = $state<HeightMode>("influence");
	let panelOpen = $state(false);
	let overlay = $state.raw<{ labels: Array<{ id: string; text: string; x: number; y: number; strong: boolean }>; ticks: Array<{ text: string; x: number; y: number }> }>({ labels: [], ticks: [] });
	const active = $derived(heightModes.find((candidate) => candidate.id === mode));

	function terrain(container: HTMLDivElement) {
		const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
		const renderer = new WebGLRenderer({ antialias: true, alpha: true });
		renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		container.prepend(renderer.domElement);
		const scene = new Scene();
		const camera = new PerspectiveCamera(42, 1, 1, 4000);
		camera.position.set(0, 120, 230);
		const controls = new OrbitControls(camera, renderer.domElement);
		controls.target.set(0, HEIGHT * 0.35, 0);
		controls.enableDamping = true;
		controls.autoRotate = !reduced;
		controls.autoRotateSpeed = 0.4;
		controls.minDistance = 40;
		controls.maxDistance = 900;
		controls.maxPolarAngle = Math.PI * 0.495;
		controls.addEventListener("start", () => {
			controls.autoRotate = false;
		});
		const uniforms = { scale: { value: 1 } };
		const material = new ShaderMaterial({ uniforms, vertexShader, fragmentShader, transparent: true, depthWrite: false });
		const geometry = new BufferGeometry();
		geometry.boundingSphere = new Sphere(new Vector3(0, HEIGHT / 2, 0), RADIUS * 1.6 + HEIGHT);
		const cloud = new Points(geometry, material);
		const webGeometry = new BufferGeometry();
		const webMaterial = new LineBasicMaterial({ vertexColors: true, transparent: true, depthWrite: false });
		const web = new LineSegments(webGeometry, webMaterial);
		const lineGeometry = new BufferGeometry();
		const lineMaterial = new LineBasicMaterial({ transparent: true, opacity: 0.75, depthWrite: false });
		const lines = new LineSegments(lineGeometry, lineMaterial);
		web.frustumCulled = false;
		lines.frustumCulled = false;
		let grid = new PolarGridHelper(RADIUS * 1.15, 12, 6, 96);
		scene.add(grid, web, lines, cloud);
		const raycaster = new Raycaster();
		raycaster.params.Points.threshold = 3;
		const pointer = new Vector2();
		const projected = new Vector3();
		const home = new Vector3(0, HEIGHT * 0.35, 0);
		const destination = home.clone();
		let layer = $state.raw<Layer | null>(null);
		let theme = $state(0);
		let pairs: Array<[Spot, Spot]> = [];
		let segments: Array<[Spot, Spot]> = [];
		let labelled: Spot[] = [];
		let hovered: Spot | null = null;
		let pressed: { x: number; y: number } | null = null;

		function drawLines() {
			const place = (edges: Array<[Spot, Spot]>) => new BufferAttribute(new Float32Array(edges.flatMap(([from, to]) => [from.x, from.height * HEIGHT, from.z, to.x, to.height * HEIGHT, to.z])), 3);
			webGeometry.setAttribute("position", place(pairs));
			lineGeometry.setAttribute("position", place(segments));
		}

		function pick(event: PointerEvent) {
			if (layer === null) return null;
			const bounds = renderer.domElement.getBoundingClientRect();
			pointer.set(((event.clientX - bounds.left) / bounds.width) * 2 - 1, -((event.clientY - bounds.top) / bounds.height) * 2 + 1);
			raycaster.setFromCamera(pointer, camera);
			for (const hit of raycaster.intersectObject(cloud)) {
				const spot = hit.index === undefined ? undefined : layer.spots.at(hit.index);
				if (spot !== undefined && spot.opacity > 0.05) return spot;
			}
			return null;
		}

		$effect(() => {
			const ground = groundLayout(graph);
			const spots = graph.nodes.flatMap((node, slot) => {
				const point = ground.get(node.id);
				return point === undefined ? [] : [{ node, slot, x: point.x * RADIUS, z: point.z * RADIUS, height: 0, target: 0, opacity: 0, stroke: new Color() }];
			});
			const positions = new BufferAttribute(new Float32Array(spots.flatMap((spot) => [spot.x, 0, spot.z])), 3);
			const tints = new BufferAttribute(new Float32Array(spots.length * 3), 3);
			const opacities = new BufferAttribute(new Float32Array(spots.length), 1);
			geometry.setAttribute("position", positions);
			geometry.setAttribute("tint", tints);
			geometry.setAttribute("opacity", opacities);
			geometry.setAttribute("size", new BufferAttribute(new Float32Array(spots.map((spot) => 1.3 + Math.sqrt(spot.node.degree) * 0.75)), 1));
			layer = { spots, index: new Map(spots.map((spot) => [spot.node.id, spot])), positions, tints, opacities };
		});

		$effect(() => {
			const current = layer;
			void theme;
			if (current === null) return;
			const styles = getComputedStyle(container);
			lineMaterial.color.setStyle(styles.getPropertyValue("--color-foreground").trim());
			const elevate = elevation(graph.nodes, mode, Date.now());
			const shown = (node: MemoryNode) => visible[node.kind] && elevate(node) !== null;
			for (const spot of current.spots) {
				const level = elevate(spot.node);
				if (level !== null) spot.target = level;
				if (reduced) spot.height = spot.target;
				const value = styles.getPropertyValue(memoryKinds[spot.node.kind].token).trim();
				const color = new Color().setStyle(value, LinearSRGBColorSpace);
				spot.stroke.setStyle(value);
				spot.opacity = !shown(spot.node) ? 0 : focus !== null && !focus.has(spot.node.id) ? 0.12 : 0.92;
				current.tints.setXYZ(spot.slot, color.r, color.g, color.b);
				current.opacities.setX(spot.slot, spot.opacity);
			}
			current.tints.needsUpdate = true;
			current.opacities.needsUpdate = true;
			const chosen = selected === null ? undefined : current.index.get(selected);
			pairs = graph.edges.flatMap((edge): Array<[Spot, Spot]> => {
				const from = current.index.get(edge.source);
				const to = current.index.get(edge.target);
				return from !== undefined && to !== undefined && shown(from.node) && shown(to.node) ? [[from, to]] : [];
			});
			webGeometry.setAttribute("color", new BufferAttribute(new Float32Array(pairs.flatMap((pair) => pair.flatMap((spot) => [spot.stroke.r, spot.stroke.g, spot.stroke.b]))), 3));
			webMaterial.opacity = chosen === undefined ? 0.28 : 0.06;
			segments =
				chosen === undefined ? [] : (
					graph.edges.flatMap((edge): Array<[Spot, Spot]> => {
						const from = current.index.get(edge.source);
						const to = current.index.get(edge.target);
						if (from === undefined || to === undefined || (from !== chosen && to !== chosen)) return [];
						return shown(from.node) && shown(to.node) ? [[from, to]] : [];
					})
				);
			drawLines();
			labelled = current.spots
				.filter((spot) => shown(spot.node))
				.sort((left, right) => right.node.influence - left.node.influence)
				.slice(0, 6);
			if (chosen === undefined) destination.copy(home);
			else destination.set(chosen.x, chosen.target * HEIGHT, chosen.z);
		});

		$effect(() => {
			void theme;
			const styles = getComputedStyle(container);
			scene.remove(grid);
			grid.dispose();
			grid = new PolarGridHelper(RADIUS * 1.15, 12, 6, 96, styles.getPropertyValue("--color-border-strong").trim(), styles.getPropertyValue("--color-border").trim());
			const materials = Array.isArray(grid.material) ? grid.material : [grid.material];
			for (const gridMaterial of materials) {
				gridMaterial.transparent = true;
				gridMaterial.opacity = 0.55;
				gridMaterial.depthWrite = false;
			}
			scene.add(grid);
		});

		let frame = 0;
		function render() {
			frame = requestAnimationFrame(render);
			const current = layer;
			if (current !== null) {
				let moving = false;
				for (const spot of current.spots) {
					const delta = spot.target - spot.height;
					if (Math.abs(delta) > 0.0005) {
						spot.height += delta * 0.1;
						moving = true;
					}
					current.positions.setY(spot.slot, spot.height * HEIGHT);
				}
				current.positions.needsUpdate = true;
				if (moving) drawLines();
			}
			controls.target.lerp(destination, reduced ? 1 : 0.08);
			controls.update();
			renderer.render(scene, camera);
			const width = renderer.domElement.clientWidth;
			const height = renderer.domElement.clientHeight;
			const screen = (x: number, y: number, z: number) => {
				projected.set(x, y, z).project(camera);
				return projected.z > 1 ? null : { x: ((projected.x + 1) / 2) * width, y: ((1 - projected.y) / 2) * height };
			};
			const ticks = active === undefined ? [] : active.ticks.flatMap((tick) => {
				const point = screen(-RADIUS * 1.15, tick.value * HEIGHT, 0);
				return point === null ? [] : [{ text: tick.text, ...point }];
			});
			const chosen = current === null || selected === null ? undefined : current.index.get(selected);
			const shown = new Set([...labelled, ...(chosen === undefined ? [] : [chosen]), ...(hovered === null ? [] : [hovered])]);
			const shownLabels = [...shown].flatMap((spot) => {
				if (spot.opacity <= 0.05) return [];
				const point = screen(spot.x, spot.height * HEIGHT, spot.z);
				const title = spot.node.title;
				return point === null ? [] : [{ id: spot.node.id, text: title.length > 26 ? `${title.slice(0, 25)}…` : title, strong: spot === hovered || spot === chosen, ...point }];
			});
			overlay = { labels: shownLabels, ticks };
		}
		render();

		const resize = new ResizeObserver(() => {
			const width = container.clientWidth;
			const height = container.clientHeight;
			renderer.setSize(width, height, false);
			camera.aspect = width / Math.max(1, height);
			camera.updateProjectionMatrix();
			uniforms.scale.value = (height * renderer.getPixelRatio()) / (2 * Math.tan((camera.fov * Math.PI) / 360));
		});
		resize.observe(container);
		const themes = new MutationObserver(() => {
			theme += 1;
		});
		themes.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
		renderer.domElement.addEventListener("pointerdown", (event) => {
			pressed = { x: event.clientX, y: event.clientY };
		});
		renderer.domElement.addEventListener("pointermove", (event) => {
			if (event.buttons === 0) hovered = pick(event);
		});
		renderer.domElement.addEventListener("pointerleave", () => {
			hovered = null;
		});
		renderer.domElement.addEventListener("pointerup", (event) => {
			if (pressed === null || Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) > 4 || layer === null) return;
			const spot = pick(event);
			onselect(spot === null ? null : spot.node.id);
			pressed = null;
		});

		return () => {
			cancelAnimationFrame(frame);
			resize.disconnect();
			themes.disconnect();
			controls.dispose();
			geometry.dispose();
			lineGeometry.dispose();
			webGeometry.dispose();
			webMaterial.dispose();
			grid.dispose();
			material.dispose();
			lineMaterial.dispose();
			renderer.dispose();
			renderer.domElement.remove();
		};
	}
</script>

<div class="terrain" {@attach terrain}>
	{#each overlay.ticks as tick (tick.text)}
		<span class="tick" style:translate={`${tick.x}px ${tick.y}px`}>{tick.text}</span>
	{/each}
	{#each overlay.labels as label (label.id)}
		<span class="label" class:strong={label.strong} style:translate={`${label.x}px ${label.y}px`}>{label.text}</span>
	{/each}
	{#if panelOpen}
		<div class="modes" role="group" aria-label="Height">
			<div class="modes-header">
				<p>What shapes height?</p>
				<button type="button" class="panel-toggle" aria-label="Hide height options" onclick={() => (panelOpen = false)}><ChevronDown size={14} /></button>
			</div>
			<div class="options">
				{#each heightModes as option (option.id)}
					<button type="button" aria-pressed={mode === option.id} onclick={() => (mode = option.id)}>{option.label}</button>
				{/each}
			</div>
			{#if active !== undefined}<strong>{active.title}</strong><span>{active.caption}</span>{/if}
		</div>
	{:else}
		<button type="button" class="reopen" aria-label="Show height options" onclick={() => (panelOpen = true)}><Mountain size={13} />{active === undefined ? "Height" : active.label}</button>
	{/if}
</div>

<style>
	.terrain { position: relative; flex: 1; min-width: 0; min-height: 0; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: radial-gradient(circle at 50% 45%, color-mix(in srgb, var(--color-muted) 55%, var(--color-background)), var(--color-background) 70%); }
	.terrain :global(canvas) { display: block; width: 100%; height: 100%; cursor: grab; touch-action: none; }
	.terrain :global(canvas:active) { cursor: grabbing; }
	.tick, .label { position: absolute; top: 0; left: 0; pointer-events: none; white-space: nowrap; font-family: var(--font-sans); }
	.tick { color: var(--color-muted-foreground); font-size: 0.6875rem; transform: translate(-100%, -50%); }
	.label { padding: 0.0625rem 0.375rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--color-background) 72%, transparent); color: var(--color-foreground); font-size: 0.6875rem; transform: translate(-50%, calc(-100% - 0.375rem)); }
	.label.strong { background: var(--color-foreground); color: var(--color-background); font-weight: 500; }
	.modes { position: absolute; right: 0.75rem; bottom: 0.75rem; display: grid; gap: 0.375rem; width: min(21rem, calc(100% - 1.5rem)); padding: 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: color-mix(in srgb, var(--color-background) 88%, transparent); box-shadow: var(--shadow-md); font-family: var(--font-sans); font-size: 0.75rem; backdrop-filter: blur(8px); }
	.modes p { margin: 0; color: var(--color-muted-foreground); }
	.modes-header { display: flex; align-items: center; justify-content: space-between; }
	.panel-toggle { display: inline-flex; padding: 0.125rem; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.panel-toggle:hover { background: var(--color-muted); color: var(--color-foreground); }
	.reopen { position: absolute; right: 0.75rem; bottom: 0.75rem; display: inline-flex; align-items: center; gap: 0.375rem; padding: 0.375rem 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-background) 88%, transparent); box-shadow: var(--shadow-sm); color: var(--color-foreground); cursor: pointer; font-family: var(--font-sans); font-size: 0.75rem; backdrop-filter: blur(8px); }
	.reopen:hover { background: var(--color-muted); }
	.options { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.25rem; }
	.options button { padding: 0.3125rem 0; border: 1px solid transparent; border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font: inherit; transition: background-color var(--duration-fast), color var(--duration-fast); }
	.options button:hover { color: var(--color-foreground); }
	.options button[aria-pressed="true"] { border-color: var(--color-border); background: var(--color-muted); color: var(--color-foreground); }
	.modes strong { margin-top: 0.125rem; font-weight: 600; }
	.modes span { color: var(--color-muted-foreground); line-height: 1.5; }
	@media (prefers-reduced-motion: reduce) {
		.options button { transition: none; }
	}
</style>
