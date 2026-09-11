<script lang="ts">
	import { ChevronUp, ListTodo, Layers } from "@lucide/svelte";
	import { onMount, tick } from "svelte";
	import { listen } from "@tauri-apps/api/event";
	import { invoke } from "@tauri-apps/api/core";
	import type { CommandResponse, IslandGeometry } from "../../consumer";
	import { widgets } from "../../dashboard";
	import type { createDashboardSession } from "./session.svelte";
	import type { createLayoutSession } from "./layout.svelte";
	import WidgetContent from "./WidgetContent.svelte";

	let { session, layoutSession }: {
		session: ReturnType<typeof createDashboardSession>;
		layoutSession: ReturnType<typeof createLayoutSession>;
	} = $props();
	const placement = $derived(layoutSession.layout?.widgets.find((item) => item.id === layoutSession.layout?.islandWidgetId) ?? null);
	let expanded = $state(false);
	let contentReady = $state(false);
	let hovered = false;
	let refreshError = $state<string | null>(null);
	let request = 0;
	let disposed = false;
	let topInset = $state(0);
	let notchWidth = $state(0);
	const isTodo = $derived(placement?.widget.kind === "planner");
	const remaining = $derived(session.todos.data?.items.filter((item) => !item.completed).length ?? null);
	let resizing = Promise.resolve();
	function resize(expanded: boolean) {
		resizing = resizing.then(async () => {
			if (disposed) return;
			const response = await invoke<CommandResponse<IslandGeometry>>("set_island_expanded", { expanded });
			if (response.status === "ready") { topInset = response.data.topInset; notchWidth = response.data.notchWidth; }
			else refreshError = response.message;
		});
		return resizing;
	}
	onMount(() => {
		void resize(false);
		const unlisten = listen("island-collapse", () => close());
		return () => { disposed = true; request += 1; void unlisten.then((stop) => stop()); };
	});
	let trigger = $state<HTMLButtonElement | null>(null);


	$effect(() => {
		const kind = placement?.widget.kind;
		if (!expanded || kind !== "planner") return;
		const timer = window.setInterval(() => { if (!session.todos.loading) void session.loadTodos(); }, 60_000);
		return () => window.clearInterval(timer);
	});

	async function expand() {
		if (expanded || placement === null) return;
		expanded = true;
		refreshError = null;
		const version = ++request;
		await resize(true);
		if (disposed || version !== request) return;
		contentReady = true;
		if (placement.widget.kind === "planner") {
			if (!session.todos.loading) await session.refreshPlanner();
			return;
		}
		const response = await invoke<CommandResponse<null>>("refresh_island");
		if (version === request && response.status === "failed") refreshError = response.message;
	}

	function close(restoreFocus = false) {
		expanded = false;
		contentReady = false;
		request += 1;
		void resize(false);
		if (restoreFocus) void tick().then(() => trigger?.focus());
	}
</script>

<svelte:window onkeydown={(event) => { if (expanded && event.key === "Escape") { event.stopPropagation(); close(true); } }} />

{#if placement !== null}
	<section class="island" class:expanded class:notched={topInset > 0} style:--island-top-inset={`${topInset}px`} style:--island-notch-width={`${notchWidth}px`} data-content-typography aria-label="Dynamic Island" onpointerenter={(event) => { if (event.pointerType !== "touch") { hovered = true; void expand(); } }} onpointerleave={(event) => { hovered = false; if (!event.currentTarget.contains(document.activeElement)) close(); }} onfocusout={(event) => { if (!hovered && (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget))) close(); }}>
		<div class="surface">
			{#if !expanded}
				<button class="trigger" bind:this={trigger} type="button" aria-label={`Expand ${widgets[placement.widget.kind].label}`} aria-expanded="false" aria-controls="dynamic-island-content" onclick={() => void expand()}>
					<span class="compact-icon">{#if isTodo}<ListTodo size={16} />{:else}<Layers size={15} />{/if}</span>
					<span class="compact-label">{widgets[placement.widget.kind].label}</span>
					<span class="compact-status">{#if isTodo && remaining !== null}{remaining}{:else}<span class="status-dot"></span>{/if}</span>
				</button>
			{:else}
				<div class="expanded-view" id="dynamic-island-content">
					<header>
						<div class="heading"><span class="heading-icon">{#if isTodo}<ListTodo size={17} />{:else}<Layers size={17} />{/if}</span><span>{widgets[placement.widget.kind].label}</span></div>
						<div class="actions">{#if isTodo}<span class="count">{remaining === null ? session.selectedDate : `${remaining} open`}</span>{/if}<button class="close" type="button" aria-label="Collapse Dynamic Island" onclick={() => close(true)}><ChevronUp size={16} /></button></div>
					</header>
					<div class="island-content">
						{#if refreshError !== null}<p role="alert">{refreshError}</p>{/if}
						{#if contentReady}{#key placement.id}<WidgetContent embedded {placement} {session} serviceCatalog={layoutSession.serviceCatalog} />{/key}{/if}
					</div>
				</div>
			{/if}
		</div>
	</section>
{/if}

<style>
	.island {
		--color-background: var(--color-island-background);
		--color-foreground: var(--color-on-dark);
		--color-muted: color-mix(in srgb, var(--color-on-dark) 8%, var(--color-island-background));
		--color-muted-foreground: color-mix(in srgb, var(--color-on-dark) 55%, var(--color-island-background));
		--color-border: color-mix(in srgb, var(--color-on-dark) 12%, var(--color-island-background));
		--color-divider: var(--color-border);
		--color-accent: var(--color-on-dark);
		--color-accent-foreground: var(--color-island-background);
		--island-shoulder: 0px;
		position: relative; width: 100%; height: 100dvh; box-sizing: border-box;
		padding-inline: var(--island-shoulder); color: var(--color-foreground);
	}
	.island.notched { --island-shoulder: 6px; }
	.island.notched.expanded { --island-shoulder: 18px; }
	.island.notched::before, .island.notched::after {
		content: ""; position: absolute; top: 0; width: var(--island-shoulder); height: var(--island-shoulder);
		background: var(--color-background);
	}
	.island.notched::before { left: 0; mask-image: radial-gradient(circle at 0 100%, transparent calc(var(--island-shoulder) - 0.5px), var(--color-on-dark) var(--island-shoulder)); }
	.island.notched::after { right: 0; mask-image: radial-gradient(circle at 100% 100%, transparent calc(var(--island-shoulder) - 0.5px), var(--color-on-dark) var(--island-shoulder)); }
	.surface { height: 100%; overflow: hidden; background: var(--color-background); border-radius: var(--radius-full); }
	.expanded .surface { border-radius: calc(var(--radius-xl) * 2.4); }
	.notched .surface { border-radius: 0 0 calc(var(--radius-lg) * 2) calc(var(--radius-lg) * 2); }
	.notched.expanded .surface { border-radius: 0 0 calc(var(--radius-xl) * 2.4) calc(var(--radius-xl) * 2.4); }
	.trigger { display: grid; grid-template-columns: 24px minmax(0, 1fr) 24px; width: 100%; height: 100%; box-sizing: border-box; align-items: center; gap: 8px; padding: 0 14px; border: 0; background: transparent; color: inherit; font-family: var(--font-sans); cursor: pointer; }
	.compact-icon, .compact-status { display: grid; place-items: center; }
	.compact-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.72rem; font-weight: 500; }
	.compact-status { color: var(--color-muted-foreground); font-size: 0.7rem; font-variant-numeric: tabular-nums; }
	.status-dot { width: 4px; height: 4px; border-radius: var(--radius-full); background: var(--color-muted-foreground); }
	.notched .trigger { grid-template-columns: minmax(24px, 1fr) var(--island-notch-width) minmax(24px, 1fr); gap: 0; padding-inline: 8px; }
	.notched .compact-label { visibility: hidden; }
	.expanded-view { display: flex; flex-direction: column; height: 100%; box-sizing: border-box; padding: max(var(--island-top-inset), 12px) 22px 20px; animation: reveal var(--duration-slow) ease-out; }
	header { display: flex; flex: 0 0 36px; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 12px; }
	.heading, .actions { display: flex; align-items: center; gap: 10px; }
	.heading { min-width: 0; font-size: 0.82rem; font-weight: 600; letter-spacing: -0.015em; }
	.heading-icon { display: grid; place-items: center; width: 30px; height: 30px; border-radius: var(--radius-full); background: var(--color-muted); }
	.count { color: var(--color-muted-foreground); font-size: 0.68rem; white-space: nowrap; font-variant-numeric: tabular-nums; }
	.close { display: grid; place-items: center; width: 28px; height: 28px; border: 0; border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.close:hover, .close:focus-visible { background: var(--color-muted); color: var(--color-foreground); }
	.island-content { container-type: inline-size; min-height: 0; overflow: auto; }
	p[role="alert"] { margin: 0 0 12px; color: var(--color-error); font-size: 0.75rem; }
	@keyframes reveal { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; transform: translateY(0); } }
	@media (prefers-reduced-motion: reduce) { .expanded-view { animation: none; } }
</style>
