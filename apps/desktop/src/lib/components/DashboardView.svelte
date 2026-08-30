<script lang="ts">
	import { Activity, ChartNoAxesCombined, Check, ChevronRight, CloudSun, Cpu, Database, HardDrive, ListTodo, MemoryStick, Network, Plus, RefreshCw, RotateCcw, Search, Settings2, Sparkles, X } from "@lucide/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import type { CherryInBalance, CodexUsage, CommandResponse, DeepSeekBalance, GithubSnapshot, OpenCodeUsage, StockReport, TaskManagerSnapshot, TodoList, WeatherReport, WidgetLayout, WidgetPlacement } from "../consumer";
	import { widgetKey, widgetOptions, widgets } from "../dashboard";
	import GithubPanel from "./GithubPanel.svelte";
	import StocksPanel from "./StocksPanel.svelte";
	import Todo from "./Todo.svelte";
	import UsagePanel from "./UsagePanel.svelte";
	import WeatherPanel from "./WeatherPanel.svelte";

	let {
		snapshot,
		error,
		refreshing,
		usage,
		usageError,
		openCodeUsage,
		openCodeUsageError,
		deepSeekBalance,
		deepSeekBalanceError,
		cherryInUsage,
		cherryInUsageError,
		weather,
		weatherError,
		stocks,
		stocksError,
		github,
		githubError,
		todos,
		todosError,
		todosLoading,
		todayDate,
		todoDate,
		onaddtodo,
		ontoggletodo,
		ondeletetodo,
		onselecttododate,
		onrefresh,
	}: {
		snapshot: TaskManagerSnapshot | null;
		error: string | null;
		refreshing: boolean;
		usage: CodexUsage | null;
		usageError: string | null;
		openCodeUsage: OpenCodeUsage | null;
		openCodeUsageError: string | null;
		deepSeekBalance: DeepSeekBalance | null;
		deepSeekBalanceError: string | null;
		cherryInUsage: CherryInBalance | null;
		cherryInUsageError: string | null;
		weather: WeatherReport | null;
		weatherError: string | null;
		stocks: StockReport | null;
		stocksError: string | null;
		github: GithubSnapshot | null;
		githubError: string | null;
		todos: TodoList | null;
		todosError: string | null;
		todosLoading: boolean;
		todayDate: string;
		todoDate: string;
		onaddtodo: (text: string) => Promise<boolean>;
		ontoggletodo: (id: string, completed: boolean) => Promise<void>;
		ondeletetodo: (id: string) => Promise<void>;
		onselecttododate: (date: string) => Promise<void>;
		onrefresh: () => void;
	} = $props();

	const percentFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	const rateFormatter = new Intl.NumberFormat("en-US", {
		maximumFractionDigits: 1,
		style: "unit",
		unit: "kilobyte-per-second",
	});
	let layout = $state<WidgetPlacement[]>([]);
	let layoutLoading = $state(true);
	let layoutSaving = $state(false);
	let layoutError = $state<string | null>(null);
	let editing = $state(false);
	let widgetLibraryOpen = $state(false);
	let widgetSearch = $state("");
	let selectedWidgetId = $state("weather");
	let weatherLocation = $state("");
	let stockSymbol = $state("");
	let widgetFormError = $state<string | null>(null);
	let draggingId = $state<string | null>(null);
	let dragChanged = $state(false);
	let layoutBeforeDrag = $state<WidgetPlacement[] | null>(null);
	let availableWidgets = $derived(widgetOptions.filter((option) => option.kind === "stock" || option.kind === "weather" || !layout.some((item) => widgetKey(item.widget) === option.id)));
	let filteredWidgets = $derived(availableWidgets.filter((option) => `${option.label} ${option.description}`.toLocaleLowerCase().includes(widgetSearch.trim().toLocaleLowerCase())));
	let selectedWidget = $derived.by(() => {
		const selected = availableWidgets.find((option) => option.id === selectedWidgetId);
		return selected ?? filteredWidgets[0] ?? null;
	});

	onMount(() => {
		void loadLayout();
	});

	async function loadLayout() {
		const response = await invoke<CommandResponse<WidgetLayout>>("read_layout");
		layoutLoading = false;
		if (response.status === "ready") {
			layout = response.data.widgets;
			layoutError = null;
		} else {
			layoutError = response.message;
		}
	}

	async function saveLayout(items: WidgetPlacement[]) {
		if (layoutSaving) return false;
		layoutSaving = true;
		const next: WidgetLayout = { widgets: items };
		const response = await invoke<CommandResponse<null>>("save_layout", { layout: next });
		layoutSaving = false;
		if (response.status === "ready") {
			layout = items;
			layoutError = null;
			return true;
		} else {
			layoutError = response.message;
			return false;
		}
	}

	function openWidgetLibrary() {
		if (availableWidgets.length === 0 || layoutSaving) return;
		const firstWidget = availableWidgets[0] ?? null;
		if (firstWidget === null) return;
		widgetSearch = "";
		weatherLocation = "";
		stockSymbol = "";
		widgetFormError = null;
		selectedWidgetId = firstWidget.id;
		widgetLibraryOpen = true;
	}

	async function addSelectedWidget() {
		if (selectedWidget === null) return;
		let placement: WidgetPlacement;
		if (selectedWidget.kind === "stock") {
			const symbol = stockSymbol.trim().toLocaleUpperCase();
			if (!/^[A-Z0-9.-]{1,12}$/.test(symbol)) {
				widgetFormError = "Enter a valid U.S. stock symbol, such as AAPL or BRK.B.";
				return;
			}
			const id = `stock-${symbol.toLocaleLowerCase()}`;
			if (layout.some((item) => widgetKey(item.widget) === id)) {
				widgetFormError = `${symbol} is already on the Dashboard.`;
				return;
			}
			placement = { id, widget: { kind: "stock", symbol } };
		} else if (selectedWidget.kind === "weather") {
			const location = weatherLocation.trim();
			if (location.length < 2 || location.length > 120) {
				widgetFormError = "Enter a city, region-qualified place, or postal code.";
				return;
			}
			if (layout.some((item) => widgetKey(item.widget) === `weather-${location.toLocaleLowerCase()}`)) {
				widgetFormError = `${location} is already on the Dashboard.`;
				return;
			}
			placement = { id: `weather-${crypto.randomUUID()}`, widget: { kind: "weather", location } };
		} else {
			placement = { id: selectedWidget.id, widget: selectedWidget.widget };
		}
		widgetFormError = null;
		if (await saveLayout([...layout, placement])) {
			widgetLibraryOpen = false;
			if (placement.widget.kind === "stock" || placement.widget.kind === "weather") onrefresh();
		}
	}

	function startDragging(event: PointerEvent, id: string) {
		if (!editing) return;
		if (event.button !== 0) return;
		if (layoutSaving) {
			event.preventDefault();
			return;
		}
		event.preventDefault();
		draggingId = id;
		dragChanged = false;
		layoutBeforeDrag = [...layout];
		if (event.currentTarget instanceof HTMLElement) event.currentTarget.setPointerCapture(event.pointerId);
	}

	function moveDraggedWidget(event: PointerEvent) {
		if (draggingId === null) return;
		const target = document.elementsFromPoint(event.clientX, event.clientY).map((element) => element.closest<HTMLElement>("[data-widget-id]")).find((element) => element !== null);
		const targetId = target?.dataset.widgetId;
		if (targetId === undefined) return;
		if (draggingId === null || draggingId === targetId) return;
		const sourceIndex = layout.findIndex((item) => item.id === draggingId);
		const targetIndex = layout.findIndex((item) => item.id === targetId);
		if (sourceIndex < 0 || targetIndex < 0) return;
		const rows: number[] = [];
		let row = 0;
		let occupiedColumns = 0;
		for (const item of layout) {
			const columns = widgets[item.widget.kind].span.columns;
			if (occupiedColumns > 0 && occupiedColumns + columns > 12) {
				row += 1;
				occupiedColumns = 0;
			}
			rows.push(row);
			occupiedColumns += columns;
			if (occupiedColumns === 12) {
				row += 1;
				occupiedColumns = 0;
			}
		}
		const next = [...layout];
		const dragged = next.find((item) => item.id === draggingId) ?? null;
		if (dragged === null) return;
		next.splice(sourceIndex, 1);
		if (rows[sourceIndex] === rows[targetIndex]) {
			next.splice(targetIndex, 0, dragged);
		} else {
			let targetRowStart = targetIndex;
			let targetRowEnd = targetIndex;
			while (targetRowStart > 0 && rows[targetRowStart - 1] === rows[targetIndex]) targetRowStart -= 1;
			while (targetRowEnd + 1 < rows.length && rows[targetRowEnd + 1] === rows[targetIndex]) targetRowEnd += 1;
			const insertionIndex = sourceIndex > targetIndex ? targetRowStart : targetRowEnd;
			next.splice(insertionIndex, 0, dragged);
		}
		layout = next;
		dragChanged = true;
	}

	async function finishDragging(event: PointerEvent) {
		if (event.currentTarget instanceof HTMLElement && event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
		draggingId = null;
		if (!dragChanged) {
			layoutBeforeDrag = null;
			return;
		}
		dragChanged = false;
		const previous = layoutBeforeDrag;
		layoutBeforeDrag = null;
		if (!(await saveLayout(layout)) && previous !== null) layout = previous;
	}

	async function resetLayout() {
		if (layoutSaving) return;
		layoutSaving = true;
		const response = await invoke<CommandResponse<WidgetLayout>>("reset_layout");
		layoutSaving = false;
		if (response.status === "ready") {
			layout = response.data.widgets;
			layoutError = null;
		} else {
			layoutError = response.message;
		}
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

<svelte:window onpointerup={(event) => void finishDragging(event)} onpointercancel={(event) => void finishDragging(event)} />

<section class="dashboard" aria-label="Dashboard">
	<header>
		<div><p>UGOS Task Manager</p><h1>Dashboard</h1></div>
		<div class="header-actions">
			{#if editing}
				<button type="button" disabled={layoutSaving} onclick={() => void resetLayout()}><RotateCcw size={14} /> 恢复默认</button>
				<button type="button" class="add-widget-button" disabled={availableWidgets.length === 0 || layoutSaving} onclick={openWidgetLibrary}><Plus size={14} /> 添加组件</button>
			{/if}
			<button type="button" class:active={editing} disabled={layoutLoading || layoutSaving} onclick={() => { editing = !editing; widgetLibraryOpen = false; }}>
				{#if editing}<Check size={14} /> 完成{:else}<Settings2 size={14} /> 编辑{/if}
			</button>
			<button type="button" disabled={refreshing} onclick={onrefresh}>
				<span class={refreshing ? "spinning" : ""}><RefreshCw size={14} /></span> Refresh
			</button>
		</div>
	</header>
	{#if editing}
		<div class="edit-hint"><span>按住组件并拖动排序；右上角按钮可删除组件。</span></div>
	{/if}

	{#if layoutError !== null}
		<div class="layout-error" role="alert">{layoutError}</div>
	{/if}
	{#if layoutLoading}
		<div class="empty-dashboard"><Activity size={18} /><span>正在读取组件布局…</span></div>
	{:else if layout.length === 0 && layoutError === null}
		<div class="empty-dashboard"><Activity size={18} /><span>还没有组件。选择“编辑”后添加。</span></div>
	{:else if layout.length > 0}
		<div class="widget-grid" class:editing>
			{#each layout as placement (placement.id)}
				{@const kind = placement.widget.kind}
				{@const widget = widgets[kind]}
				<section
					class={`widget widget-${widget.id} columns-${widget.span.columns}`}
					class:dragging={draggingId === placement.id}
					data-widget-id={placement.id}
					aria-label={`${widget.label}组件`}
					onpointerdown={(event) => startDragging(event, placement.id)}
					onpointermove={moveDraggedWidget}
				>
					{#if editing}
						<button
							type="button"
							class="widget-delete"
							disabled={layoutSaving}
							aria-label={`删除${widget.label}组件`}
							title="删除组件"
							onpointerdown={(event) => event.stopPropagation()}
							onclick={(event) => {
								event.stopPropagation();
								void saveLayout(layout.filter((item) => item.id !== placement.id));
							}}
						><X size={11} /></button>
					{/if}
					{#if kind === "cpu"}
						<article class="metric">
							<h2><Cpu size={15} /> CPU <small>Live</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.cpu}<p><strong>{percentFormatter.format(snapshot.cpu.usedPercent)}%</strong><span>{percentFormatter.format(snapshot.cpu.temperature)} °C</span></p><svg class="sparkline cpu-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="CPU usage and temperature trends"><polyline points={chartPoints(snapshot.cpuHistory.map((point) => point.usedPercent))}></polyline><polyline class="secondary" points={chartPoints(snapshot.cpuHistory.map((point) => point.temperature))}></polyline></svg>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "memory"}
						<article class="metric">
							<h2><MemoryStick size={15} /> Memory <small>Live</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.memory}<p><strong>{percentFormatter.format(snapshot.memory.usedPercent)}%</strong><span>used</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Memory usage trend"><polyline points={chartPoints(snapshot.memoryHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "storage"}
						<article class="metric">
							<h2><Database size={15} /> Storage <small>Capacity</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.storage}<p><strong>{percentFormatter.format(snapshot.storage.usedPercent)}%</strong><span>{percentFormatter.format(100 - snapshot.storage.usedPercent)}% free</span></p><div class="capacity" role="progressbar" aria-label="Storage used capacity" aria-valuenow={snapshot.storage.usedPercent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${snapshot.storage.usedPercent}%`}></span></div><div class="capacity-labels"><span>Used</span><span>Free</span></div>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "network"}
						<article class="metric">
							<h2><Network size={15} /> Network <small>Live</small></h2>
							{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if snapshot?.network}<p><strong>↓ {rateFormatter.format(snapshot.network.receiveRate / 1000)}</strong><span>↑ {rateFormatter.format(snapshot.network.sendRate / 1000)}</span></p><svg class="sparkline network-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Network receive and send trend"><polyline points={chartPoints(snapshot.networkHistory.map((point) => point.receiveRate), "zero")}></polyline><polyline class="secondary" points={chartPoints(snapshot.networkHistory.map((point) => point.sendRate), "zero")}></polyline></svg>{:else}<p class="metric-message">Connecting to UGOS…</p>{/if}
						</article>
					{:else if kind === "weather"}
						<WeatherPanel {weather} location={placement.widget.location} error={weatherError} />
					{:else if kind === "stock"}
						<StocksPanel {stocks} symbol={placement.widget.symbol} error={stocksError} />
					{:else if kind === "github"}
						<GithubPanel {github} error={githubError} />
					{:else if kind === "todo"}
						<Todo {todos} error={todosError} loading={todosLoading} {todayDate} selectedDate={todoDate} onadd={onaddtodo} ontoggle={ontoggletodo} ondelete={ondeletetodo} onselect={onselecttododate} />
					{:else}
						<UsagePanel codex={usage} codexError={usageError} openCode={openCodeUsage} openCodeError={openCodeUsageError} deepSeek={deepSeekBalance} deepSeekError={deepSeekBalanceError} cherryIn={cherryInUsage} cherryInError={cherryInUsageError} />
					{/if}
				</section>
			{/each}
		</div>
	{/if}
</section>

{#if widgetLibraryOpen}
	<div class="widget-library-backdrop" role="presentation" onclick={(event) => { if (event.currentTarget === event.target) widgetLibraryOpen = false; }} onkeydown={(event) => { if (event.key === "Escape") widgetLibraryOpen = false; }}>
		<div class="widget-library" role="dialog" aria-modal="true" aria-labelledby="widget-library-title">
			<header class="library-header">
				<div><span>组件库</span><h2 id="widget-library-title">添加组件</h2></div>
				<button type="button" class="icon-button" aria-label="关闭组件库" onclick={() => (widgetLibraryOpen = false)}><X size={16} /></button>
			</header>
			<div class="library-body">
				<aside>
					<label class="widget-search"><Search size={14} /><input aria-label="搜索组件" placeholder="搜索组件" bind:value={widgetSearch} /></label>
					<div class="widget-list">
						{#each filteredWidgets as option (option.id)}
							<button type="button" class:selected={selectedWidget?.id === option.id} onclick={() => { selectedWidgetId = option.id; widgetFormError = null; }}>
								<span class="library-list-icon">{#if option.kind === "cpu"}<Cpu size={16} />{:else if option.kind === "memory"}<MemoryStick size={16} />{:else if option.kind === "storage"}<HardDrive size={16} />{:else if option.kind === "network"}<Network size={16} />{:else if option.kind === "todo"}<ListTodo size={16} />{:else if option.kind === "stock"}<ChartNoAxesCombined size={16} />{:else if option.kind === "weather"}<CloudSun size={16} />{:else}<Sparkles size={16} />{/if}</span>
								<span>{option.label}<small>{option.category === "system" ? "系统" : option.category === "personal" ? "个人" : "在线服务"}</small></span>
								<ChevronRight size={13} />
							</button>
						{/each}
						{#if filteredWidgets.length === 0}<p>没有匹配的组件</p>{/if}
					</div>
				</aside>
				{#if selectedWidget !== null}
					<div class="widget-preview-pane">
						<div class="preview-copy"><span>{selectedWidget.category === "system" ? "系统" : selectedWidget.category === "personal" ? "个人" : "在线服务"}</span><h3>{selectedWidget.label}</h3><p>{selectedWidget.description}</p></div>
					<div class="widget-preview">
						<div class="preview-card">
							<span class="preview-app-icon">{#if selectedWidget.kind === "cpu"}<Cpu size={22} />{:else if selectedWidget.kind === "memory"}<MemoryStick size={22} />{:else if selectedWidget.kind === "storage"}<HardDrive size={22} />{:else if selectedWidget.kind === "network"}<Network size={22} />{:else if selectedWidget.kind === "todo"}<ListTodo size={22} />{:else if selectedWidget.kind === "stock"}<ChartNoAxesCombined size={22} />{:else if selectedWidget.kind === "weather"}<CloudSun size={22} />{:else}<Sparkles size={22} />{/if}</span>
							<div><strong>{selectedWidget.label}</strong><span>{selectedWidget.description}</span></div>
							<div class="preview-lines"><i></i><i></i><i></i></div>
						</div>
					</div>
					{#if selectedWidget.kind === "weather"}
						<label class="widget-config-field"><span>地点</span><input placeholder="例如 杭州、Paris, France 或 10001" maxlength="120" bind:value={weatherLocation} oninput={() => (widgetFormError = null)} /></label>
					{:else if selectedWidget.kind === "stock"}
						<label class="widget-config-field"><span>美股代码</span><input class="stock-symbol-input" placeholder="例如 AAPL、TSLA、BRK.B" maxlength="12" bind:value={stockSymbol} oninput={() => (widgetFormError = null)} /></label>
					{/if}
					{#if widgetFormError !== null}<p class="widget-form-error" role="alert">{widgetFormError}</p>{/if}
						<div class="library-footer">
							<button type="button" class="primary-button" disabled={layoutSaving} onclick={() => void addSelectedWidget()}><Plus size={14} /> 添加组件</button>
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.dashboard {
		display: flex;
		width: 100%;
		min-width: 64rem;
		min-height: calc(100vh - 9rem);
		box-sizing: border-box;
		flex-direction: column;
		margin: 0 auto;
	}

	header {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	header p,
	header h1 { margin: 0; }
	header p { margin-bottom: 0.35rem; color: var(--color-accent); font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	header h1 { font-family: var(--font-serif); font-size: 2rem; font-weight: 500; }
	.header-actions { display: flex; align-items: center; justify-content: flex-end; gap: 0.45rem; }

	button {
		display: inline-flex;
		height: 2rem;
		align-items: center;
		gap: 0.4rem;
		padding: 0 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-background);
		color: var(--color-muted-foreground);
		cursor: pointer;
		font-size: 0.75rem;
		transition: transform var(--duration-base) cubic-bezier(0.16, 1, 0.3, 1), border-color var(--duration-base) cubic-bezier(0.16, 1, 0.3, 1);
	}

	button:hover:not(:disabled) { transform: translateY(-1px); border-color: var(--color-accent); }
	button:disabled { cursor: not-allowed; opacity: 0.5; }
	button.active { border-color: var(--color-accent); color: var(--color-accent); }
	button span { display: inline-flex; }
	.spinning { animation: spin var(--duration-spinner) linear infinite; }
	.add-widget-button { border-color: var(--color-accent); color: var(--color-accent); }

	.empty-dashboard {
		display: flex;
		min-height: 7rem;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-muted-foreground);
		font-size: 0.8rem;
	}
	.layout-error { padding: 0.7rem 0.85rem; margin-bottom: 0.75rem; border: 1px solid var(--color-error); border-radius: var(--radius-md); color: var(--color-error); font-size: 0.72rem; }
	.edit-hint { display: flex; align-items: center; gap: 0.4rem; padding: 0.55rem 0.7rem; margin: -0.75rem 0 1rem; border-radius: var(--radius-md); background: color-mix(in srgb, var(--color-accent) 8%, transparent); color: var(--color-muted-foreground); font-size: 0.68rem; }

	.widget-grid {
		display: grid;
		grid-auto-flow: row;
		grid-template-columns: repeat(12, minmax(0, 1fr));
		gap: 0.75rem;
	}

	.widget {
		position: relative;
		display: flex;
		min-width: 0;
		container-type: inline-size;
		animation: card-enter var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1) both;
	}
	.widget.columns-3 { grid-column: span 3; }
	.widget.columns-4 { grid-column: span 4; }
	.widget.columns-6 { grid-column: span 6; }
	.widget.columns-8 { grid-column: span 8; }
	.widget.columns-12 { grid-column: span 12; }
	.widget-grid.editing .widget { border-radius: var(--radius-lg); outline: 1px dashed var(--color-accent); outline-offset: 3px; cursor: move; touch-action: none; user-select: none; }
	.widget-grid.editing .widget :global(*) { cursor: move !important; }
	.widget.dragging { z-index: 5; opacity: 0.35; }
	.widget-grid.editing .widget .widget-delete { position: absolute; z-index: 10; top: 0.35rem; right: 0.35rem; display: grid; width: 1.35rem; height: 1.35rem; place-items: center; padding: 0; border-color: var(--color-border); border-radius: var(--radius-full); background: var(--color-background); box-shadow: var(--shadow-xs); cursor: pointer !important; }

	.metric {
		width: 100%;
		overflow: hidden;
		box-sizing: border-box;
		padding: 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
		transition: transform var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), border-color var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), box-shadow var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1);
	}
	.metric:hover { transform: translateY(-2px); border-color: var(--color-accent); box-shadow: var(--shadow-sm); }
	.widget :global(.context-panel),
	.widget :global(.stocks-panel),
	.widget :global(.github-panel),
	.widget :global(.todo),
	.widget :global(.usage-panel) { width: 100%; box-sizing: border-box; margin-top: 0; }

	h2 { display: flex; align-items: center; gap: 0.4rem; margin: 0 0 0.65rem; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	h2 small { margin-left: auto; color: var(--color-accent); font-family: var(--font-mono); font-size: 0.5rem; letter-spacing: 0.08em; }
	.metric p { display: flex; align-items: baseline; justify-content: space-between; gap: 0.75rem; margin: 0.35rem 0; color: var(--color-muted-foreground); font-size: 0.7rem; }
	.metric p.metric-message { display: flex; min-height: 5rem; align-items: center; justify-content: center; line-height: 1.4; text-align: center; }
	.metric strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 1rem; }
	.sparkline { display: block; width: 100%; height: 2.75rem; margin-top: 0.55rem; overflow: visible; }
	.sparkline polyline { fill: none; stroke: var(--color-accent); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; vector-effect: non-scaling-stroke; transition: points var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.cpu-chart .secondary,
	.network-chart .secondary { stroke: var(--color-warning); opacity: 0.75; }
	.capacity { height: 0.65rem; margin-top: 1.25rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.capacity span { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); transition: width var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.capacity-labels { display: flex; justify-content: space-between; margin-top: 0.45rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.5rem; text-transform: uppercase; }

	.widget-library-backdrop { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; box-sizing: border-box; padding: 2rem; background: var(--color-overlay); backdrop-filter: blur(10px); }
	.widget-library { width: min(52rem, calc(100vw - 4rem)); height: min(38rem, calc(100vh - 4rem)); overflow: hidden; border: 1px solid var(--color-border); border-radius: calc(var(--radius-lg) * 1.5); background: var(--color-background); box-shadow: var(--shadow-lg); }
	.library-header { display: flex; height: 4.5rem; align-items: center; justify-content: space-between; box-sizing: border-box; padding: 0 1.25rem 0 1.5rem; margin: 0; border-bottom: 1px solid var(--color-border); }
	.library-header div > span { color: var(--color-muted-foreground); font-size: 0.62rem; }
	.library-header h2 { margin: 0.1rem 0 0; color: var(--color-foreground); font-size: 1.05rem; font-weight: 600; text-transform: none; }
	.icon-button { width: 1.9rem; justify-content: center; padding: 0; border-radius: var(--radius-full); background: var(--color-muted); }
	.library-body { display: grid; height: calc(100% - 4.5rem); grid-template-columns: 15rem minmax(0, 1fr); }
	.library-body aside { padding: 1rem; border-right: 1px solid var(--color-border); background: color-mix(in srgb, var(--color-muted) 45%, transparent); }
	.widget-search { display: flex; height: 2rem; align-items: center; gap: 0.45rem; box-sizing: border-box; padding: 0 0.6rem; border: 1px solid transparent; border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); box-shadow: var(--shadow-xs); }
	.widget-search:focus-within { border-color: var(--color-accent); }
	.widget-search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--color-foreground); font: inherit; font-size: 0.72rem; }
	.widget-list { display: grid; gap: 0.2rem; margin-top: 0.85rem; }
	.widget-list > button { width: 100%; height: 3rem; justify-content: flex-start; padding: 0 0.55rem; border-color: transparent; background: transparent; text-align: left; }
	.widget-list > button:hover:not(:disabled) { transform: none; background: var(--color-background); }
	.widget-list > button.selected { border-color: color-mix(in srgb, var(--color-accent) 30%, transparent); background: color-mix(in srgb, var(--color-accent) 10%, transparent); color: var(--color-foreground); }
	.widget-list > button > span:nth-child(2) { display: grid; flex: 1; gap: 0.1rem; }
	.widget-list small { color: var(--color-muted-foreground); font-size: 0.55rem; }
	.widget-list p { padding: 1.5rem 0; margin: 0; color: var(--color-muted-foreground); font-size: 0.7rem; text-align: center; }
	.library-list-icon { display: grid; width: 1.9rem; height: 1.9rem; place-items: center; border-radius: var(--radius-md); background: var(--color-background); color: var(--color-accent); box-shadow: var(--shadow-xs); }
	.widget-preview-pane { display: grid; min-width: 0; grid-template-rows: auto 1fr auto auto; padding: 2rem 2.25rem 1.5rem; }
	.preview-copy > span { color: var(--color-accent); font-size: 0.62rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; }
	.preview-copy h3 { margin: 0.3rem 0 0.35rem; font-family: var(--font-serif); font-size: 1.65rem; font-weight: 500; }
	.preview-copy p { margin: 0; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.widget-preview { display: grid; place-items: center; }
	.preview-card { position: relative; display: grid; width: 13rem; min-height: 10rem; grid-template-rows: auto 1fr auto; box-sizing: border-box; padding: 1.1rem; overflow: hidden; border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent); border-radius: calc(var(--radius-lg) * 1.4); background: linear-gradient(145deg, color-mix(in srgb, var(--color-accent) 11%, var(--color-background)), var(--color-background)); box-shadow: var(--shadow-lg); transition: width var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1); }
	.preview-app-icon { display: grid; width: 2.5rem; height: 2.5rem; place-items: center; border-radius: var(--radius-lg); background: var(--color-accent); color: var(--color-accent-foreground); box-shadow: var(--shadow-sm); }
	.preview-card > div:nth-child(2) { display: grid; align-content: end; gap: 0.2rem; margin-top: 1rem; }
	.preview-card strong { font-size: 1rem; }
	.preview-card div > span { color: var(--color-muted-foreground); font-size: 0.63rem; }
	.preview-lines { display: flex; gap: 0.3rem; margin-top: 0.8rem; }
	.preview-lines i { width: 2.5rem; height: 0.25rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-accent) 35%, var(--color-muted)); }
	.preview-lines i:nth-child(2) { width: 1.75rem; }
	.preview-lines i:nth-child(3) { width: 3.25rem; }
	.library-footer { display: flex; align-items: center; justify-content: flex-end; gap: 1rem; }
	.primary-button { border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.widget-config-field { display: grid; gap: 0.35rem; margin-bottom: 0.45rem; color: var(--color-muted-foreground); font-size: 0.62rem; }
	.widget-config-field input { height: 2.2rem; box-sizing: border-box; padding: 0 0.65rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font: inherit; }
	.widget-config-field input:focus { border-color: var(--color-accent); }
	.stock-symbol-input { text-transform: uppercase; }
	.widget-form-error { margin: 0 0 0.45rem; color: var(--color-error); font-size: 0.62rem; }

	@keyframes spin { to { rotate: 360deg; } }
	@keyframes card-enter { from { transform: translateY(8px); opacity: 0; } }

	@media (prefers-reduced-motion: reduce) {
		button,
		.widget,
		.sparkline polyline,
		.capacity span { animation: none; transition: none; }
		button:hover:not(:disabled),
		.metric:hover { transform: none; }
	}
</style>
