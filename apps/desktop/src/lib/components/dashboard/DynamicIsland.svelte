<script lang="ts">
	import { ArrowLeftRight, ChartNoAxesCombined, ChevronDown, ChevronUp, CloudSun, Cpu, Gamepad2, Gauge, GitBranch, HardDrive, ListTodo, Layers, MemoryStick, Network, Pin, Quote, ShieldCheck, WalletCards } from "@lucide/svelte";
	import { onMount, tick } from "svelte";
	import { listen } from "@tauri-apps/api/event";
	import { invoke } from "@tauri-apps/api/core";
	import type { CommandResponse, IslandGeometry, WidgetKind } from "../../consumer";
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
	let hovered = $state(false);
	let intentTimer: ReturnType<typeof setTimeout> | null = null;
	let surface = $state<HTMLElement | null>(null);
	let collapseButton = $state<HTMLButtonElement | null>(null);
	const icons: Record<WidgetKind, typeof Layers> = {
		invalid: Layers, planner: ListTodo, spending: WalletCards, game: Gamepad2, steam: Gamepad2,
		cpu: Cpu, localCpu: Cpu, memory: MemoryStick, localMemory: MemoryStick,
		storage: HardDrive, localStorage: HardDrive, network: Network, localNetwork: Network,
		weather: CloudSun, stock: ChartNoAxesCombined, exchange: ArrowLeftRight,
		serviceStatus: ShieldCheck, github: GitBranch, quotation: Quote,
		codex: Gauge, openCode: Gauge, claude: Gauge, codexClaude: Gauge, grok: Gauge, copilot: Gauge,
		deepSeek: WalletCards, cherryIn: WalletCards, tokenFlux: Gauge, dimAgent: Gauge,
	};
	const Icon = $derived(placement === null ? Layers : icons[placement.widget.kind]);

	function clearIntent() {
		if (intentTimer !== null) clearTimeout(intentTimer);
		intentTimer = null;
	}

	function leave() {
		hovered = false;
		clearIntent();
		intentTimer = setTimeout(() => {
			intentTimer = null;
			if (!surface?.contains(document.activeElement)) close();
		}, 280);
	}
	let refreshError = $state<string | null>(null);
	let request = 0;
	let disposed = false;
	let topInset = $state(0);
	let notchWidth = $state(0);
	const isPlanner = $derived(placement?.widget.kind === "planner");
	const remaining = $derived(session.todos.data?.date === session.selectedDate ? session.todos.data.items.filter((item) => !item.completed).length : null);
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
		return () => { clearIntent(); disposed = true; request += 1; void unlisten.then((stop) => stop()); };
	});
	let trigger = $state<HTMLButtonElement | null>(null);


	$effect(() => {
		const kind = placement?.widget.kind;
		if (!expanded || kind !== "planner") return;
		const timer = window.setInterval(() => { if (!session.todos.loading) void session.loadTodos(); }, 60_000);
		return () => window.clearInterval(timer);
	});

	async function expand(focus = false) {
		clearIntent();
		if (expanded || placement === null) return;
		expanded = true;
		refreshError = null;
		const version = ++request;
		await resize(true);
		if (disposed || version !== request) return;
		contentReady = true;
		if (focus) {
			await tick();
			if (disposed || version !== request) return;
			collapseButton?.focus();
		}
		if (placement.widget.kind === "planner") {
			if (!session.todos.loading) await session.refreshPlanner();
			return;
		}
		const response = await invoke<CommandResponse<null>>("refresh_island");
		if (version === request && response.status === "failed") refreshError = response.message;
	}

	function close(restoreFocus = false) {
		clearIntent();
		if (!expanded) return;
		expanded = false;
		contentReady = false;
		request += 1;
		void resize(false);
		if (restoreFocus) void tick().then(() => trigger?.focus());
	}
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape" && (expanded || intentTimer !== null)) { event.stopPropagation(); close(true); } }} />

{#if placement !== null}
	<section bind:this={surface} class="island" class:expanded class:hovered class:notched={topInset > 0} style:--island-top-inset={`${topInset}px`} style:--island-notch-width={`${notchWidth}px`} data-content-typography aria-label="Dynamic Island"
		onpointerenter={(event) => {
			if (event.pointerType === "touch") return;
			hovered = true;
			clearIntent();
			if (!expanded) intentTimer = setTimeout(() => { intentTimer = null; void expand(); }, 160);
		}}
		onpointerleave={leave}
		onfocusin={clearIntent}
		onfocusout={(event) => { if (!hovered && (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget))) leave(); }}>
		<div class="surface">
			{#if !expanded}
				<button class="trigger" bind:this={trigger} type="button" aria-label={`Expand ${widgets[placement.widget.kind].label}`} aria-expanded="false" aria-controls="dynamic-island-content" onclick={() => void expand(true)}>
					<span class="compact-icon"><Icon size={16} /></span>
					<span class="compact-label">{widgets[placement.widget.kind].label}</span>
					<span class="compact-status">{#if isPlanner && remaining !== null}{remaining}{:else}<ChevronDown size={12} />{/if}</span>
				</button>
			{:else}
				<div class="expanded-view" id="dynamic-island-content">
					<header>
						<div class="heading"><span class="heading-icon"><Icon size={18} /></span><div class="heading-copy"><span class="eyebrow">Vesper <span> / </span> Dynamic Island</span><h2>{widgets[placement.widget.kind].label}</h2></div></div>
						<div class="actions">{#if isPlanner}<span class="count">{remaining === null ? session.selectedDate : `${remaining} open`}</span>{/if}<button class="close" bind:this={collapseButton} type="button" aria-label="Collapse Dynamic Island" onclick={() => close(true)}><ChevronUp size={16} /></button></div>
					</header>
					<div class="island-content" aria-busy={!contentReady}>
						{#if refreshError !== null}<p role="alert">{refreshError}</p>{/if}
						{#if contentReady}{#key placement.id}<WidgetContent embedded {placement} {session} serviceCatalog={layoutSession.serviceCatalog} />{/key}{:else}<div class="opening" role="status">Opening widget…<span></span><span></span></div>{/if}
					</div>
					<footer><span><Pin size={10} /> Pinned from Dashboard</span><span>Esc to collapse</span></footer>
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
		--color-muted-foreground: color-mix(in srgb, var(--color-on-dark) 65%, var(--color-island-background));
		--color-border: color-mix(in srgb, var(--color-on-dark) 12%, var(--color-island-background));
		--color-divider: var(--color-border);
		--color-accent: var(--color-on-dark);
		--color-accent-foreground: var(--color-island-background);
		--island-shoulder: 0px;
		position: relative; width: 100%; height: 100dvh; box-sizing: border-box;
		padding-inline: var(--island-shoulder); color: var(--color-foreground); color-scheme: dark;
	}
	.island.notched { --island-shoulder: 6px; }
	.island.notched.expanded { --island-shoulder: 18px; }
	.island.notched::before, .island.notched::after {
		content: ""; position: absolute; top: 0; width: var(--island-shoulder); height: var(--island-shoulder);
		background: var(--color-background);
	}
	.island.notched::before { left: 0; mask-image: radial-gradient(circle at 0 100%, transparent calc(var(--island-shoulder) - 0.5px), var(--color-on-dark) var(--island-shoulder)); }
	.island.notched::after { right: 0; mask-image: radial-gradient(circle at 100% 100%, transparent calc(var(--island-shoulder) - 0.5px), var(--color-on-dark) var(--island-shoulder)); }
	.surface { height: 100%; overflow: hidden; background: var(--color-background); border-radius: var(--radius-full); transition: border-radius var(--duration-slow) ease; }
	.expanded .surface { border-radius: calc(var(--radius-xl) * 2.8); }
	.notched .surface { border-radius: 0 0 calc(var(--radius-lg) * 2) calc(var(--radius-lg) * 2); }
	.notched.expanded .surface { border-radius: 0 0 calc(var(--radius-xl) * 2.8) calc(var(--radius-xl) * 2.8); }
	.trigger { display: grid; grid-template-columns: 24px minmax(0, 1fr) 24px; width: 100%; height: 100%; box-sizing: border-box; align-items: center; gap: 8px; padding: 0 14px; border: 0; background: transparent; color: inherit; font-family: var(--font-sans); cursor: pointer; }
	.compact-icon, .compact-status { display: grid; place-items: center; }
	.compact-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.72rem; font-weight: 500; }
	.compact-status { color: var(--color-muted-foreground); font-size: 0.7rem; font-variant-numeric: tabular-nums; }
	.hovered .compact-icon { color: var(--color-accent); }
	.trigger:focus-visible { outline: 2px solid var(--color-foreground); outline-offset: -4px; border-radius: inherit; }
	.notched .trigger { grid-template-columns: minmax(24px, 1fr) var(--island-notch-width) minmax(24px, 1fr); gap: 0; padding-inline: 8px; }
	.notched .compact-label { visibility: hidden; }
	.expanded-view { display: flex; flex-direction: column; height: 100%; box-sizing: border-box; padding: calc(var(--island-top-inset) + 16px) 22px 12px; animation: reveal var(--duration-slow) ease-out; }
	header { display: flex; flex: 0 0 42px; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 16px; }
	.heading, .actions { display: flex; align-items: center; gap: 10px; }
	.heading { min-width: 0; font-size: 0.82rem; font-weight: 600; letter-spacing: -0.015em; }
	.heading-icon { display: grid; place-items: center; width: 38px; height: 38px; border-radius: var(--radius-full); background: var(--color-muted); }
	.count { color: var(--color-muted-foreground); font-size: 0.68rem; white-space: nowrap; font-variant-numeric: tabular-nums; }
	.close { display: grid; place-items: center; width: 30px; height: 30px; border: 0; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-muted-foreground); cursor: pointer; }
	.close:hover, .close:focus-visible { background: var(--color-muted); color: var(--color-foreground); }
	.island-content { container-type: inline-size; min-height: 0; flex: 1; overflow-x: hidden; overflow-y: auto; overscroll-behavior: contain; scrollbar-width: none; }
	.island-content::-webkit-scrollbar, .island-content :global(*::-webkit-scrollbar) { display: none; width: 0; height: 0; }
	.island-content :global(*) { scrollbar-width: none; }
	.heading-copy { min-width: 0; display: grid; gap: 4px; }
	.eyebrow { color: var(--color-muted-foreground); font-size: 0.6rem; font-weight: 400; letter-spacing: 0.03em; }
	.eyebrow span { margin: 0 3px; opacity: 0.5; }
	h2 { margin: 0; font-size: 0.88rem; font-weight: 600; line-height: 1.2; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.count { padding: 5px 8px; border-radius: var(--radius-full); background: var(--color-muted); }
	footer { display: flex; justify-content: space-between; gap: 12px; flex-shrink: 0; padding-top: 12px; margin-top: 12px; border-top: 1px solid var(--color-border); color: var(--color-muted-foreground); font-size: 0.6rem; }
	footer span { display: flex; align-items: center; gap: 5px; }
	.opening { display: grid; gap: 12px; padding: 8px 0; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.opening span { height: 42px; border-radius: var(--radius-lg); background: var(--color-muted); }
	.opening span:last-child { width: 70%; }
	.close:focus-visible { outline: 2px solid var(--color-foreground); outline-offset: 3px; }
	p[role="alert"] { margin: 0 0 12px; color: var(--color-error); font-size: 0.75rem; }
	@keyframes reveal { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; transform: translateY(0); } }
	@media (prefers-reduced-motion: reduce) { .expanded-view { animation: none; } .surface { transition: none; } }
</style>
