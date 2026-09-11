<script lang="ts">
	import InvalidWidget from "./InvalidWidget.svelte";
	import { untrack } from "svelte";
	import { Cpu, Database, MemoryStick, Network } from "@lucide/svelte";
	import type { Habit, WidgetPlacement, ServiceStatusCatalogEntry } from "../../consumer";
	import type { createDashboardSession } from "./session.svelte";
	import GithubPanel from "./GithubPanel.svelte";
	import ExchangePanel from "./ExchangePanel.svelte";
	import ServiceStatusPanel from "./ServiceStatusPanel.svelte";
	import StocksPanel from "./StocksPanel.svelte";
	import StoragePanel from "./StoragePanel.svelte";
	import HabitsPanel from "./HabitsPanel.svelte";
	import Todo from "./Todo.svelte";
	import SpendingPanel from "../ledger/SpendingPanel.svelte";
	import CalendarPanel from "./CalendarPanel.svelte";
	import QuotationPanel from "./QuotationPanel.svelte";
	import UsagePanel from "./UsagePanel.svelte";
	import WeatherPanel from "./WeatherPanel.svelte";
	import GamePanel from "../games/GamePanel.svelte";
	import SteamGamesPanel from "../games/SteamGamesPanel.svelte";
	let { placement, session, serviceCatalog, embedded = false, onhabitschange = null }: {
		placement: WidgetPlacement;
		session: ReturnType<typeof createDashboardSession>;
		serviceCatalog: ServiceStatusCatalogEntry[];
		embedded?: boolean;
		onhabitschange?: ((habits: Habit[]) => Promise<boolean>) | null;
	} = $props();
	const kind = $derived(placement.widget.kind);
	const snapshot = $derived(session.dashboard.taskManager.data);
	const error = $derived(session.dashboard.taskManager.error);
	const deviceTelemetry = $derived(session.dashboard.deviceTelemetry.data);
	const deviceTelemetryError = $derived(session.dashboard.deviceTelemetry.error);
	const usage = $derived(session.dashboard.codex.data);
	const usageError = $derived(session.dashboard.codex.error);
	const openCodeUsage = $derived(session.dashboard.openCode.data);
	const openCodeUsageError = $derived(session.dashboard.openCode.error);
	const claudeUsage = $derived(session.dashboard.claude.data);
	const claudeUsageError = $derived(session.dashboard.claude.error);
	const grokUsage = $derived(session.dashboard.grok.data);
	const grokUsageError = $derived(session.dashboard.grok.error);
	const copilotUsage = $derived(session.dashboard.copilot.data);
	const copilotUsageError = $derived(session.dashboard.copilot.error);
	const deepSeekBalance = $derived(session.dashboard.deepSeek.data);
	const deepSeekBalanceError = $derived(session.dashboard.deepSeek.error);
	const cherryInUsage = $derived(session.dashboard.cherryIn.data);
	const cherryInUsageError = $derived(session.dashboard.cherryIn.error);
	const weather = $derived(session.dashboard.weather.data);
	const weatherError = $derived(session.dashboard.weather.error);
	const stocks = $derived(session.dashboard.stocks.data);
	const stocksError = $derived(session.dashboard.stocks.error);
	const exchange = $derived(session.dashboard.exchange.data);
	const exchangeError = $derived(session.dashboard.exchange.error);
	const serviceStatus = $derived(session.dashboard.serviceStatus.data);
	const serviceStatusError = $derived(session.dashboard.serviceStatus.error);
	const github = $derived(session.dashboard.github.data);
	const githubError = $derived(session.dashboard.github.error);
	const quotation = $derived(session.dashboard.quotation.data);
	const quotationError = $derived(session.dashboard.quotation.error);
	const todos = $derived(session.todos.data?.date === session.selectedDate ? session.todos.data : null);
	const todosError = $derived(session.todos.error);
	const todosLoading = $derived(session.todos.loading);
	const todayDate = $derived(session.todayDate);
	const selectedDate = $derived(session.selectedDate);
	const onaddtodo = $derived(session.addTodo);
	const ontoggletodo = $derived(session.toggleTodo);
	const ondeletetodo = $derived(session.deleteTodo);
	$effect(() => {
		if (kind !== "planner" || !selectedDate) return;
		untrack(() => { void session.loadTodos(); });
	});
	const percentFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	const rateFormatter = new Intl.NumberFormat("en-US", {
		maximumFractionDigits: 1,
		style: "unit",
		unit: "kilobyte-per-second",
	});
	const byteFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	function bytesLabel(bytes: number): string {
		return `${byteFormatter.format(bytes / 1_000_000_000)} GB`;
	}

	function chartPoints(values: Array<number | null>, scale: "adaptive" | "zero" = "adaptive"): string {
		const samples = values.filter((value): value is number => value !== null);
		if (samples.length === 0) return "";
		const sampleMinimum = Math.min(...samples);
		const sampleMaximum = Math.max(...samples);
		const padding = scale === "zero" ? Math.max((sampleMaximum - sampleMinimum) * 0.15, 2) : Math.max((sampleMaximum - sampleMinimum) * 0.15, 0.02);
		const minimum = scale === "zero" ? 0 : Math.max(0, sampleMinimum - padding);
		const minimumRange = scale === "zero" ? 1 : 0.05;
		const maximum = Math.max(minimum + minimumRange, sampleMaximum + padding);
		if (samples.length === 1) {
			return samples
				.map((value) => {
					const y = 40 - Math.min(1, Math.max(0, (value - minimum) / (maximum - minimum))) * 34;
					return `0,${y.toFixed(1)} 160,${y.toFixed(1)}`;
				})
				.join("");
		}
		const points = samples.map((value, index) => {
			const x = (index / (samples.length - 1)) * 160;
			const y = 40 - Math.min(1, Math.max(0, (value - minimum) / (maximum - minimum))) * 34;
			return `${x.toFixed(1)},${y.toFixed(1)}`;
		});
		return points.join(" ");
	}

</script>

<div class="widget-content" class:embedded>
					{#if placement.widget.kind === "invalid"}
						<InvalidWidget configuration={placement.widget.configuration} error={placement.widget.error} />
					{:else if kind === "cpu"}
						<article class="metric">
							<h2><Cpu size={15} /> UGREEN CPU <small>Live</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.cpu}<p><strong>{percentFormatter.format(snapshot.cpu.usedPercent)}%</strong><span>{percentFormatter.format(snapshot.cpu.temperature)} °C</span></p><svg class="sparkline cpu-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="CPU usage and temperature trends"><polyline points={chartPoints(snapshot.cpuHistory.map((point) => point.usedPercent))}></polyline><polyline class="secondary" points={chartPoints(snapshot.cpuHistory.map((point) => point.temperature))}></polyline></svg>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "memory"}
						<article class="metric">
							<h2><MemoryStick size={15} /> UGREEN Memory <small>Live</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.memory}<p><strong>{percentFormatter.format(snapshot.memory.usedPercent)}%</strong><span>used</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Memory usage trend"><polyline points={chartPoints(snapshot.memoryHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "storage"}
						<article class="metric">
							<h2><Database size={15} /> UGREEN Storage <small>Capacity</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.storage}<p><strong>{percentFormatter.format(snapshot.storage.usedPercent)}%</strong><span>{percentFormatter.format(100 - snapshot.storage.usedPercent)}% free</span></p><div class="capacity" role="progressbar" aria-label="Storage used capacity" aria-valuenow={snapshot.storage.usedPercent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${snapshot.storage.usedPercent}%`}></span></div><div class="capacity-labels"><span>Used</span><span>Free</span></div>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "network"}
						<article class="metric">
							<h2><Network size={15} /> UGREEN Network <small>Live</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.network}<p><strong>↓ {rateFormatter.format(snapshot.network.receiveRate / 1000)}</strong><span>↑ {rateFormatter.format(snapshot.network.sendRate / 1000)}</span></p><svg class="sparkline network-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Network receive and send trend"><polyline points={chartPoints(snapshot.networkHistory.map((point) => point.receiveRate), "zero")}></polyline><polyline class="secondary" points={chartPoints(snapshot.networkHistory.map((point) => point.sendRate), "zero")}></polyline></svg>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "localCpu"}
						<article class="metric">
							<h2><Cpu size={15} /> Device CPU <small>Live</small></h2>
							{#if deviceTelemetryError !== null}<p class="metric-message" role="alert">{deviceTelemetryError}</p>{:else if deviceTelemetry !== null}<p><strong>{percentFormatter.format(deviceTelemetry.cpu.usedPercent)}%</strong><span>used</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Current-device CPU usage trend"><polyline points={chartPoints(deviceTelemetry.cpuHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p class="metric-message">Reading current device…</p>{/if}
						</article>
					{:else if kind === "localMemory"}
						<article class="metric">
							<h2><MemoryStick size={15} /> Device Memory <small>Live</small></h2>
							{#if deviceTelemetryError !== null}<p class="metric-message" role="alert">{deviceTelemetryError}</p>{:else if deviceTelemetry !== null}<p><strong>{percentFormatter.format(deviceTelemetry.memory.usedPercent)}%</strong><span>{bytesLabel(deviceTelemetry.memory.usedBytes)} / {bytesLabel(deviceTelemetry.memory.totalBytes)}</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Current-device memory usage trend"><polyline points={chartPoints(deviceTelemetry.memoryHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p class="metric-message">Reading current device…</p>{/if}
						</article>
					{:else if kind === "localStorage"}
						<StoragePanel storage={deviceTelemetry === null ? null : deviceTelemetry.storage} error={deviceTelemetryError} />
					{:else if kind === "localNetwork"}
						<article class="metric">
							<h2><Network size={15} /> Device Network <small>Live</small></h2>
							{#if deviceTelemetryError !== null}<p class="metric-message" role="alert">{deviceTelemetryError}</p>{:else if deviceTelemetry !== null}<p><strong>↓ {rateFormatter.format(deviceTelemetry.network.receiveRate / 1000)}</strong><span>↑ {rateFormatter.format(deviceTelemetry.network.sendRate / 1000)}</span></p><svg class="sparkline network-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Current-device network receive and send trend"><polyline points={chartPoints(deviceTelemetry.networkHistory.map((point) => point.receiveRate), "zero")}></polyline><polyline class="secondary" points={chartPoints(deviceTelemetry.networkHistory.map((point) => point.sendRate), "zero")}></polyline></svg>{:else}<p class="metric-message">Reading current device…</p>{/if}
						</article>
					{:else if placement.widget.kind === "weather"}
						<WeatherPanel {weather} location={placement.widget.location} error={weatherError} />
					{:else if placement.widget.kind === "stock"}
						<StocksPanel {stocks} symbol={placement.widget.symbol} error={stocksError} />
					{:else if kind === "exchange"}
						<ExchangePanel report={exchange} error={exchangeError} />
					{:else if placement.widget.kind === "serviceStatus"}
						<ServiceStatusPanel report={serviceStatus} catalog={serviceCatalog} serviceId={placement.widget.serviceId} error={serviceStatusError} />
					{:else if kind === "github"}
						<GithubPanel {github} error={githubError} />
					{:else if kind === "spending"}
						<SpendingPanel {selectedDate} {todayDate} onselect={session.selectDate} {embedded} />
					{:else if placement.widget.kind === "planner"}
						<section class="planner" class:compact={embedded} aria-label="Daily Planner">
							<div class="planner-calendar"><CalendarPanel {todayDate} {selectedDate} onselect={session.selectDate} /></div>
							<div class="planner-todos"><Todo {embedded} {todos} error={todosError} loading={todosLoading} {selectedDate} onadd={onaddtodo} onedit={session.editTodo} ontoggle={ontoggletodo} ondelete={ondeletetodo} /></div>
							<HabitsPanel {selectedDate} habits={placement.widget.habits} onchange={onhabitschange} />
						</section>
					{:else if kind === "codex" || kind === "openCode" || kind === "claude" || kind === "grok" || kind === "copilot" || kind === "deepSeek" || kind === "cherryIn"}
						<UsagePanel provider={kind} codex={usage} codexError={usageError} openCode={openCodeUsage} openCodeError={openCodeUsageError} claude={claudeUsage} claudeError={claudeUsageError} grok={grokUsage} grokError={grokUsageError} copilot={copilotUsage} copilotError={copilotUsageError} deepSeek={deepSeekBalance} deepSeekError={deepSeekBalanceError} cherryIn={cherryInUsage} cherryInError={cherryInUsageError} />
					{:else if kind === "quotation"}
						<QuotationPanel {quotation} error={quotationError} />
					{:else if placement.widget.kind === "game"}
						<GamePanel game={placement.widget.game} />
					{:else if kind === "steam"}
						<SteamGamesPanel />
					{/if}
</div>
<style>
	.planner { display: grid; grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr) minmax(0, 1.2fr); overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.planner > div { min-width: 0; border-right: 1px solid var(--color-border); }
	.planner > div > :global(section) { border: 0; box-shadow: none; border-radius: 0; margin: 0; }
	.planner.compact { grid-template-columns: 1fr; max-height: 32rem; overflow-y: auto; }
	.planner.compact > div { border-right: 0; border-bottom: 1px solid var(--color-border); }

	.widget-content { display: flex; width: 100%; min-width: 0; }
	.widget-content.embedded > :global(section), .widget-content.embedded > :global(article) { border: 0; padding: 0; background: transparent; box-shadow: none; }
	.widget-content > :global(section), .widget-content > :global(article) { width: 100%; box-sizing: border-box; margin-top: 0; }
	.metric {
		width: 100%;
		overflow: hidden;
		box-sizing: border-box;
		padding: 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
		transition: transform var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), border-color var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), box-shadow var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1);
	}
	.metric:hover { transform: translateY(-2px); border-color: var(--color-accent); box-shadow: var(--shadow-sm); }

	h2 { display: flex; align-items: center; gap: 0.4rem; margin: 0 0 0.65rem; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	h2 small { margin-left: auto; color: var(--color-accent); font-family: var(--font-mono); font-size: 0.5rem; letter-spacing: 0.08em; }
	.metric p { display: flex; align-items: baseline; justify-content: space-between; gap: 0.75rem; margin: 0.35rem 0; color: var(--color-muted-foreground); font-size: 0.7rem; }
	.metric p.metric-message { display: flex; min-height: 5rem; align-items: center; justify-content: center; line-height: 1.4; text-align: center; }
	.metric strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 1rem; }
	.sparkline { display: block; width: 100%; height: 2.25rem; margin-top: 0.4rem; overflow: visible; }
	.sparkline polyline { fill: none; stroke: var(--color-accent); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; vector-effect: non-scaling-stroke; transition: points var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.cpu-chart .secondary,
	.network-chart .secondary { stroke: var(--color-warning); opacity: 0.75; }
	.capacity { height: 0.65rem; margin-top: 0.75rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.capacity span { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); transition: width var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.capacity-labels { display: flex; justify-content: space-between; margin-top: 0.45rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.5rem; text-transform: uppercase; }

	@media (prefers-reduced-motion: reduce) { .metric, .sparkline polyline, .capacity span { transition: none; } .metric:hover { transform: none; } }
</style>
