<script lang="ts">
	import type { StarRailReport } from "../../consumer";
	let { report }: { report: StarRailReport } = $props();
	const total = $derived(report.pools.reduce((sum, pool) => sum + pool.total, 0));
</script>

<div class="distribution">
	<svg viewBox="0 0 100 100" role="img" aria-label={`Pulls by pool: ${report.pools.map((pool) => `${pool.name}: ${pool.total}`).join("; ")}`}>
		<circle class="track" cx="50" cy="50" r="39" />
		{#if total > 0}{#each report.pools as pool, index (pool.id)}{#if pool.total > 0}
			<circle class={`segment pool-${index}`} cx="50" cy="50" r="39" pathLength="100" stroke-dasharray={`${pool.total / total * 100} 100`} stroke-dashoffset={-report.pools.slice(0, index).reduce((sum, item) => sum + item.total, 0) / total * 100} />
		{/if}{/each}{/if}
		<text x="50" y="49" class="total">{total}</text><text x="50" y="63" class="caption">pulls</text>
	</svg>
	<table class="pool-totals">
		<thead><tr><th>Pool</th><th>Pulls</th><th>Since 5★</th></tr></thead>
		<tbody>{#each report.pools as pool, index (pool.id)}
			<tr><td><span class={`dot pool-${index}`}></span>{pool.name}</td><td>{pool.total}</td><td>{pool.sinceHighRarity === null ? "—" : pool.sinceHighRarity}</td></tr>
		{/each}</tbody>
	</table>
</div>
<details class="report-details">
	<summary>Saved 5★ records</summary>
	<p class="muted coverage">miHoYo totals and 5★ records. Individual 3★ and 4★ records are not provided.</p>
	<table>
		<thead><tr><th>Item</th><th>Pool</th><th>Pulls to 5★</th></tr></thead>
		<tbody>{#each report.pools as pool (pool.id)}{#each pool.fiveStars as star (star.id)}
			<tr><td>{star.name}{#if star.isUp}<span class="muted"> · UP</span>{/if}</td><td>{pool.name}</td><td>{star.pulls}</td></tr>
		{/each}{/each}</tbody>
	</table>
</details>

<style>
	.report-details .coverage { margin: 0.35rem 0; font-size: 0.65rem; }
	.distribution { display: grid; grid-template-columns: 4.5rem minmax(0, 1fr); align-items: center; gap: 0.6rem; margin: 0.25rem 0; }
	.distribution svg { width: 100%; overflow: visible; }
	circle { fill: none; stroke-width: 9; }
	.track { stroke: var(--color-muted); }
	.segment { stroke: currentColor; transform: rotate(-90deg); transform-origin: 50% 50%; }
	.pool-0 { color: var(--color-accent); }
	.pool-1 { color: var(--color-warning); }
	.pool-2 { color: var(--color-success); }
	.pool-3 { color: var(--color-error); }
	.pool-4 { color: var(--color-foreground); }
	.pool-5 { color: var(--color-muted-foreground); }
	.dot { display: inline-block; width: 0.35rem; height: 0.35rem; margin-right: 0.3rem; border-radius: 50%; background: currentColor; }
	text { text-anchor: middle; fill: var(--color-foreground); font-family: var(--font-mono); }
	.total { font-size: 17px; font-weight: 600; }
	.caption { font-size: 9px; fill: var(--color-muted-foreground); }
	.distribution .pool-totals { font-size: 0.65rem; line-height: 1.35; }
	.pool-totals th, .pool-totals td { padding: 0.15rem 0.2rem; border: 0; }
	.pool-totals th { color: var(--color-muted-foreground); font-weight: 400; }
	.pool-totals td:not(:first-child), .pool-totals th:not(:first-child) { width: 1%; white-space: nowrap; text-align: right; font-variant-numeric: tabular-nums; }
	.muted { color: var(--color-muted-foreground); }
	details { font-size: 0.65rem; }
	.report-details summary { margin: 0.25rem 0 0; cursor: pointer; color: var(--color-muted-foreground); }
	table { width: 100%; border-collapse: collapse; text-align: left; }
	th, td { padding: 0.3rem; border-bottom: 1px solid var(--color-border); }
</style>
