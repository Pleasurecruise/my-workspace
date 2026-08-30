<script lang="ts">
	import { ChartNoAxesCombined, TrendingDown, TrendingUp } from "@lucide/svelte";
	import type { StockReport } from "../consumer";

	let { stocks, symbol, error }: { stocks: StockReport | null; symbol: string; error: string | null } = $props();
	let stock = $derived(stocks?.stocks.find((item) => item.symbol === symbol) ?? null);
	let failure = $derived(stocks?.failures.find((item) => item.symbol === symbol) ?? null);
	const priceFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 2 });

	function chartPoints(values: number[]): string {
		if (values.length === 0) return "";
		const minimum = Math.min(...values);
		const maximum = Math.max(...values);
		const range = Math.max(maximum - minimum, 0.01);
		return values.map((value, index) => `${((index / Math.max(values.length - 1, 1)) * 100).toFixed(1)},${(28 - ((value - minimum) / range) * 24).toFixed(1)}`).join(" ");
	}
</script>

<section class="stocks-panel" aria-label="股票行情">
	<header><span><ChartNoAxesCombined size={15} /> {symbol}</span><small>最近一个月</small></header>
	{#if stock !== null}
		<div class="stock-heading"><strong>{stock.name}</strong><span>{stock.exchange} · {stock.currency}</span></div>
		<svg viewBox="0 0 100 32" preserveAspectRatio="none" role="img" aria-label={`${stock.symbol} 最近一个月价格走势`}><polyline class:negative={stock.change < 0} points={chartPoints(stock.points.map((point) => point.close))}></polyline></svg>
		<div class="quote"><strong>{priceFormatter.format(stock.price)}</strong><span class:negative={stock.change < 0}>{#if stock.change < 0}<TrendingDown size={12} />{:else}<TrendingUp size={12} />{/if}{stock.changePercent >= 0 ? "+" : ""}{stock.changePercent.toFixed(2)}%</span></div>
	{:else if error !== null || failure !== null}
		<p class="message" role="alert">{error ?? failure?.message}</p>
	{:else}
		<p class="message">正在读取股票行情…</p>
	{/if}
</section>

<style>
	.stocks-panel { width: 100%; min-width: 0; box-sizing: border-box; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header,
	header span,
	.quote,
	.quote span { display: flex; align-items: center; }
	header { justify-content: space-between; color: var(--color-muted-foreground); }
	header span { gap: 0.4rem; font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
	header small { color: var(--color-accent); font-family: var(--font-mono); font-size: 0.5rem; }
	.stock-heading { display: grid; min-width: 0; gap: 0.08rem; margin-top: 0.75rem; }
	.stock-heading strong { font-family: var(--font-mono); font-size: 0.72rem; }
	.stock-heading span { overflow: hidden; color: var(--color-muted-foreground); font-size: 0.52rem; text-overflow: ellipsis; white-space: nowrap; }
	svg { width: 100%; height: 2rem; margin: 0.45rem 0; overflow: visible; }
	polyline { fill: none; stroke: var(--color-success); stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.5; vector-effect: non-scaling-stroke; }
	polyline.negative { stroke: var(--color-error); }
	.quote { justify-content: space-between; gap: 0.3rem; }
	.quote > strong { font-family: var(--font-mono); font-size: 0.72rem; font-weight: 500; }
	.quote span { gap: 0.14rem; color: var(--color-success); font-family: var(--font-mono); font-size: 0.52rem; }
	.quote span.negative { color: var(--color-error); }
	.message { margin: 1rem 0 0; color: var(--color-muted-foreground); font-size: 0.68rem; }
</style>
