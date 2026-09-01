<script lang="ts">
	import { ChartNoAxesCombined, TrendingDown, TrendingUp } from "@lucide/svelte";
	import type { StockReport } from "../consumer";

	let { stocks, symbol, error }: { stocks: StockReport | null; symbol: string; error: string | null } = $props();
	let stock = $derived.by(() => {
		if (stocks === null) return null;
		for (const item of stocks.stocks) {
			if (item.symbol === symbol) return item;
		}
		return null;
	});
	let failure = $derived.by(() => {
		if (stocks === null) return null;
		for (const item of stocks.failures) {
			if (item.symbol === symbol) return item;
		}
		return null;
	});
	const priceFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 2 });

	function chart(values: StockReport["stocks"][number]["points"]): { line: string; area: string; endX: number; endY: number } {
		const closes = values.map((point) => point.close);
		const minimum = Math.min(...closes);
		const maximum = Math.max(...closes);
		const range = Math.max(maximum - minimum, 0.01);
		const first = { x: 0, y: 27 - ((values[0].close - minimum) / range) * 23 };
		let line = `M${first.x.toFixed(2)} ${first.y.toFixed(2)}`;
		let current = first;
		for (const [offset, point] of values.slice(1).entries()) {
			const index = offset + 1;
			const next = { x: (index / (values.length - 1)) * 100, y: 27 - ((point.close - minimum) / range) * 23 };
			const middle = (current.x + next.x) / 2;
			line += ` C${middle.toFixed(2)} ${current.y.toFixed(2)},${middle.toFixed(2)} ${next.y.toFixed(2)},${next.x.toFixed(2)} ${next.y.toFixed(2)}`;
			current = next;
		}
		return { line, area: `${line} L${current.x.toFixed(2)} 32 L0 32 Z`, endX: current.x, endY: current.y };
	}

	let gradientId = $derived(`stock-gradient-${symbol.replaceAll(/[^a-zA-Z0-9]/g, "-")}`);
</script>

<section class="stocks-panel" aria-label="Stock quote">
	<header><span><ChartNoAxesCombined size={15} /> {symbol}</span><small>Past month</small></header>
	{#if stock !== null}
		{@const graph = chart(stock.points)}
		<div class="stock-heading"><strong>{stock.name}</strong><span>{stock.exchange} · {stock.currency}</span></div>
		<div class="chart"><svg viewBox="0 0 100 32" preserveAspectRatio="none" role="img" aria-label={`${stock.symbol} price trend over the past month`}><defs><linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="currentColor" stop-opacity="0.22"></stop><stop offset="1" stop-color="currentColor" stop-opacity="0"></stop></linearGradient></defs><path class:negative={stock.change < 0} class="area" d={graph.area} fill={`url(#${gradientId})`}></path><path class:negative={stock.change < 0} class="line" d={graph.line}></path><circle class:negative={stock.change < 0} cx={graph.endX} cy={graph.endY} r="1.15"></circle></svg><div class="chart-range"><span>1M</span><span>{priceFormatter.format(stock.points[0].close)}</span><span>{priceFormatter.format(stock.price)}</span></div></div>
		<div class="quote"><strong>{priceFormatter.format(stock.price)}</strong><span class:negative={stock.change < 0}>{#if stock.change < 0}<TrendingDown size={12} />{:else}<TrendingUp size={12} />{/if}{stock.changePercent >= 0 ? "+" : ""}{stock.changePercent.toFixed(2)}%</span></div>
	{:else if error !== null}
		<p class="message" role="alert">{error}</p>
	{:else if failure !== null}
		<p class="message" role="alert">{failure.message}</p>
	{:else}
		<p class="message">Loading stock quote…</p>
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
	.chart { margin: 0.55rem 0 0.4rem; color: var(--color-success); }
	.chart svg { display: block; width: 100%; height: 2.75rem; overflow: visible; }
	.chart path.line { fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.65; vector-effect: non-scaling-stroke; }
	.chart path.area { stroke: none; }
	.chart path.negative { color: var(--color-error); }
	.chart circle { fill: var(--color-background); stroke: currentColor; stroke-width: 1.5; vector-effect: non-scaling-stroke; }
	.chart circle.negative { color: var(--color-error); }
	.chart-range { display: flex; justify-content: space-between; margin-top: 0.2rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.48rem; }
	.quote { justify-content: space-between; gap: 0.3rem; }
	.quote > strong { font-family: var(--font-mono); font-size: 0.95rem; font-weight: 500; letter-spacing: -0.03em; }
	.quote span { gap: 0.14rem; color: var(--color-success); font-family: var(--font-mono); font-size: 0.52rem; }
	.quote span.negative { color: var(--color-error); }
	.message { margin: 1rem 0 0; color: var(--color-muted-foreground); font-size: 0.68rem; }
</style>
