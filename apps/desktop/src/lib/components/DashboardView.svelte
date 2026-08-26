<script lang="ts">
	import { Activity, Cpu, Database, MemoryStick, Network, RefreshCw } from "@lucide/svelte";
	import type { CherryInBalance, CodexUsage, DeepSeekBalance, GithubSnapshot, OpenCodeUsage, TaskManagerSnapshot, TodoList, WeatherReport } from "../consumer";
	import GithubPanel from "./GithubPanel.svelte";
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

<section class="dashboard" aria-label="UGOS task manager">
	<header>
		<div><p>UGOS Task Manager</p><h1>Dashboard</h1></div>
		<button type="button" disabled={refreshing} onclick={onrefresh}>
			<span class={refreshing ? "spinning" : ""}><RefreshCw size={14} /></span> Refresh
		</button>
	</header>

	{#if error !== null}
		<div class="message" role="alert"><strong>Task Manager unavailable</strong><span>{error}</span></div>
	{:else if snapshot === null}
		<div class="message"><Activity size={18} /><span>Connecting to UGOS…</span></div>
	{:else}
		<div class="metrics">
			<section>
				<h2><Cpu size={15} /> CPU <small>Live</small></h2>
				{#if snapshot.cpu}<p><strong>{percentFormatter.format(snapshot.cpu.usedPercent)}%</strong><span>{percentFormatter.format(snapshot.cpu.temperature)} °C</span></p><svg class="sparkline cpu-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="CPU usage and temperature trends"><polyline points={chartPoints(snapshot.cpuHistory.map((point) => point.usedPercent))}></polyline><polyline class="secondary" points={chartPoints(snapshot.cpuHistory.map((point) => point.temperature))}></polyline></svg>{:else}<p>No CPU sample</p>{/if}
			</section>
			<section>
				<h2><MemoryStick size={15} /> Memory <small>Live</small></h2>
				{#if snapshot.memory}<p><strong>{percentFormatter.format(snapshot.memory.usedPercent)}%</strong><span>used</span></p><svg class="sparkline" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Memory usage trend"><polyline points={chartPoints(snapshot.memoryHistory.map((point) => point.usedPercent))}></polyline></svg>{:else}<p>No memory sample</p>{/if}
			</section>
			<section>
				<h2><Database size={15} /> Storage <small>Capacity</small></h2>
				{#if snapshot.storage}<p><strong>{percentFormatter.format(snapshot.storage.usedPercent)}%</strong><span>{percentFormatter.format(100 - snapshot.storage.usedPercent)}% free</span></p><div class="capacity" role="progressbar" aria-label="Storage used capacity" aria-valuenow={snapshot.storage.usedPercent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${snapshot.storage.usedPercent}%`}></span></div><div class="capacity-labels"><span>Used</span><span>Free</span></div>{:else}<p>No storage sample</p>{/if}
			</section>
			<section>
				<h2><Network size={15} /> Network <small>Live</small></h2>
				{#if snapshot.network}<p><strong>↓ {rateFormatter.format(snapshot.network.receiveRate / 1000)}</strong><span>↑ {rateFormatter.format(snapshot.network.sendRate / 1000)}</span></p><svg class="sparkline network-chart" viewBox="0 0 160 44" preserveAspectRatio="none" role="img" aria-label="Network receive and send trend"><polyline points={chartPoints(snapshot.networkHistory.map((point) => point.receiveRate), "zero")}></polyline><polyline class="secondary" points={chartPoints(snapshot.networkHistory.map((point) => point.sendRate), "zero")}></polyline></svg>{:else}<p>No network sample</p>{/if}
			</section>
		</div>
	{/if}

	<div class="weather-slot"><WeatherPanel {weather} error={weatherError} /></div>

	<div class="github-slot"><GithubPanel {github} error={githubError} /></div>

	<div class="dashboard-lower">
		<Todo
			{todos}
			error={todosError}
			loading={todosLoading}
			{todayDate}
			selectedDate={todoDate}
			onadd={onaddtodo}
			ontoggle={ontoggletodo}
			ondelete={ondeletetodo}
			onselect={onselecttododate}
		/>
		<UsagePanel
			codex={usage}
			codexError={usageError}
			openCode={openCodeUsage}
			openCodeError={openCodeUsageError}
			deepSeek={deepSeekBalance}
			deepSeekError={deepSeekBalanceError}
			cherryIn={cherryInUsage}
			cherryInError={cherryInUsageError}
		/>
	</div>
</section>

<style>
	.dashboard {
		display: flex;
		width: min(100%, 64rem);
		min-height: calc(100vh - 9rem);
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
	button:disabled { cursor: wait; opacity: 0.5; }
	button span { display: inline-flex; }
	.spinning { animation: spin var(--duration-spinner) linear infinite; }

	.message {
		display: flex;
		min-height: 7rem;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-muted-foreground);
	}

	.message strong,
	.message span { font-size: 0.8rem; }

	.metrics {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 10rem), 1fr));
		gap: 0.75rem;
	}

	.metrics section {
		position: relative;
		overflow: hidden;
		padding: 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
		transition: transform var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), border-color var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), box-shadow var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1);
	}

	.metrics,
	.weather-slot,
	.github-slot,
	.dashboard-lower { animation: card-enter var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1) both; }
	.weather-slot { animation-delay: 45ms; }
	.github-slot { animation-delay: 90ms; }
	.dashboard-lower { animation-delay: 135ms; }
	.metrics section:hover { transform: translateY(-2px); border-color: var(--color-accent); box-shadow: var(--shadow-sm); }

	h2 { display: flex; align-items: center; gap: 0.4rem; margin: 0 0 0.65rem; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	h2 small { margin-left: auto; color: var(--color-accent); font-family: var(--font-mono); font-size: 0.5rem; letter-spacing: 0.08em; }
	.metrics p { display: flex; align-items: baseline; justify-content: space-between; gap: 0.75rem; margin: 0.35rem 0; color: var(--color-muted-foreground); font-size: 0.7rem; }
	.metrics strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 1rem; }
	.sparkline { display: block; width: 100%; height: 2.75rem; margin-top: 0.55rem; overflow: visible; }
	.sparkline polyline { fill: none; stroke: var(--color-accent); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; vector-effect: non-scaling-stroke; transition: points var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.cpu-chart .secondary,
	.network-chart .secondary { stroke: var(--color-warning); opacity: 0.75; }
	.capacity { height: 0.65rem; margin-top: 1.25rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.capacity span { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); transition: width var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.capacity-labels { display: flex; justify-content: space-between; margin-top: 0.45rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.5rem; text-transform: uppercase; }

	.dashboard-lower {
		display: grid;
		grid-template-columns: minmax(16rem, 0.72fr) minmax(0, 1.28fr);
		align-items: stretch;
		gap: 0.75rem;
		margin-top: 0.75rem;
	}

	@media (max-width: 720px) {
		.dashboard-lower { grid-template-columns: 1fr; }
	}

	@keyframes spin { to { rotate: 360deg; } }
	@keyframes card-enter { from { transform: translateY(8px); opacity: 0; } }

	@media (prefers-reduced-motion: reduce) {
		button,
		.metrics,
		.weather-slot,
		.github-slot,
		.dashboard-lower,
		.sparkline polyline,
		.capacity span { animation: none; transition: none; }
		button:hover:not(:disabled),
		.metrics section:hover { transform: none; }
	}
</style>
