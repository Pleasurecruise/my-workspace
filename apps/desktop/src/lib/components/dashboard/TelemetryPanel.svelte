<script lang="ts">
	import { Cpu, Database, MemoryStick, Network } from "@lucide/svelte";
	import type { DeviceTelemetrySnapshot, TaskManagerSnapshot, QueryState } from "../../consumer";
	import StoragePanel from "./StoragePanel.svelte";
	let source:
		| { origin: "ugos"; kind: "cpu" | "memory" | "storage" | "network"; state: QueryState<TaskManagerSnapshot> }
		| { origin: "device"; kind: "localCpu" | "localMemory" | "localStorage" | "localNetwork"; state: QueryState<DeviceTelemetrySnapshot> } = $props();
	const kind = $derived(source.kind);
	const snapshot = $derived(source.origin === "ugos" ? source.state.data : null);
	const deviceTelemetry = $derived(source.origin === "device" ? source.state.data : null);
	const error = $derived(source.state.error);
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

{#if kind === "cpu"}
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
		{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if deviceTelemetry !== null}<p><strong>{percentFormatter.format(deviceTelemetry.cpu.usedPercent)}%</strong><span>used</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Current-device CPU usage trend"><polyline points={chartPoints(deviceTelemetry.cpuHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p class="metric-message">Reading current device…</p>{/if}
	</article>
{:else if kind === "localMemory"}
	<article class="metric">
		<h2><MemoryStick size={15} /> Device Memory <small>Live</small></h2>
		{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if deviceTelemetry !== null}<p><strong>{percentFormatter.format(deviceTelemetry.memory.usedPercent)}%</strong><span>{bytesLabel(deviceTelemetry.memory.usedBytes)} / {bytesLabel(deviceTelemetry.memory.totalBytes)}</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Current-device memory usage trend"><polyline points={chartPoints(deviceTelemetry.memoryHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p class="metric-message">Reading current device…</p>{/if}
	</article>
{:else if kind === "localStorage"}
	<StoragePanel storage={deviceTelemetry === null ? null : deviceTelemetry.storage} error={error} />
{:else if kind === "localNetwork"}
	<article class="metric">
		<h2><Network size={15} /> Device Network <small>Live</small></h2>
		{#if error !== null}<p class="metric-message" role="alert">{error}</p>{:else if deviceTelemetry !== null}<p><strong>↓ {rateFormatter.format(deviceTelemetry.network.receiveRate / 1000)}</strong><span>↑ {rateFormatter.format(deviceTelemetry.network.sendRate / 1000)}</span></p><svg class="sparkline network-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Current-device network receive and send trend"><polyline points={chartPoints(deviceTelemetry.networkHistory.map((point) => point.receiveRate), "zero")}></polyline><polyline class="secondary" points={chartPoints(deviceTelemetry.networkHistory.map((point) => point.sendRate), "zero")}></polyline></svg>{:else}<p class="metric-message">Reading current device…</p>{/if}
	</article>
{/if}

<style>
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
