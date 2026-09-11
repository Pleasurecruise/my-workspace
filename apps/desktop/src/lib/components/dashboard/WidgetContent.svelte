<script lang="ts">
	import type { Habit, WidgetPlacement, ServiceStatusCatalogEntry } from "../../consumer";
	import type { createDashboardSession } from "./session.svelte";
	import InvalidWidget from "./InvalidWidget.svelte";
	import TelemetryPanel from "./TelemetryPanel.svelte";
	import PlannerPanel from "./PlannerPanel.svelte";
	import GithubPanel from "./GithubPanel.svelte";
	import ExchangePanel from "./ExchangePanel.svelte";
	import ServiceStatusPanel from "./ServiceStatusPanel.svelte";
	import StocksPanel from "./StocksPanel.svelte";
	import SpendingPanel from "../ledger/SpendingPanel.svelte";
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
</script>

<div class="widget-content" class:embedded>
	{#if placement.widget.kind === "invalid"}
		<InvalidWidget configuration={placement.widget.configuration} error={placement.widget.error} />
	{:else if kind === "cpu" || kind === "memory" || kind === "storage" || kind === "network"}
		<TelemetryPanel origin="ugos" {kind} state={session.dashboard.taskManager} />
	{:else if kind === "localCpu" || kind === "localMemory" || kind === "localStorage" || kind === "localNetwork"}
		<TelemetryPanel origin="device" {kind} state={session.dashboard.deviceTelemetry} />
	{:else if placement.widget.kind === "weather"}
		<WeatherPanel weather={session.dashboard.weather.data} location={placement.widget.location} error={session.dashboard.weather.error} />
	{:else if placement.widget.kind === "stock"}
		<StocksPanel stocks={session.dashboard.stocks.data} symbol={placement.widget.symbol} error={session.dashboard.stocks.error} />
	{:else if placement.widget.kind === "exchange"}
		<ExchangePanel report={session.dashboard.exchange.data} error={session.dashboard.exchange.error} />
	{:else if placement.widget.kind === "serviceStatus"}
		<ServiceStatusPanel report={session.dashboard.serviceStatus.data} catalog={serviceCatalog} serviceId={placement.widget.serviceId} error={session.dashboard.serviceStatus.error} />
	{:else if placement.widget.kind === "github"}
		<GithubPanel github={session.dashboard.github.data} error={session.dashboard.github.error} />
	{:else if placement.widget.kind === "spending"}
		<SpendingPanel selectedDate={session.selectedDate} todayDate={session.todayDate} onselect={session.selectDate} {embedded} />
	{:else if placement.widget.kind === "planner"}
		<PlannerPanel {session} habits={placement.widget.habits} onchange={onhabitschange} {embedded} />
	{:else if kind === "codex"}
		<UsagePanel provider="codex" state={session.dashboard.codex} />
	{:else if kind === "openCode"}
		<UsagePanel provider="openCode" state={session.dashboard.openCode} />
	{:else if kind === "claude"}
		<UsagePanel provider="claude" state={session.dashboard.claude} />
	{:else if kind === "grok"}
		<UsagePanel provider="grok" state={session.dashboard.grok} />
	{:else if kind === "copilot"}
		<UsagePanel provider="copilot" state={session.dashboard.copilot} />
	{:else if kind === "deepSeek"}
		<UsagePanel provider="deepSeek" state={session.dashboard.deepSeek} />
	{:else if kind === "cherryIn"}
		<UsagePanel provider="cherryIn" state={session.dashboard.cherryIn} />
	{:else if kind === "quotation"}
		<QuotationPanel quotation={session.dashboard.quotation.data} error={session.dashboard.quotation.error} />
	{:else if placement.widget.kind === "game"}
		<GamePanel game={placement.widget.game} />
	{:else if kind === "steam"}
		<SteamGamesPanel />
	{/if}
</div>

<style>
	.widget-content { display: flex; width: 100%; min-width: 0; }
	.widget-content.embedded > :global(section), .widget-content.embedded > :global(article) { border: 0; padding: 0; background: transparent; box-shadow: none; }
	.widget-content > :global(section), .widget-content > :global(article) { width: 100%; box-sizing: border-box; margin-top: 0; }
</style>
