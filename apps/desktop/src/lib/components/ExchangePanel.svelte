<script lang="ts">
	import { ArrowLeftRight, TrendingDown, TrendingUp } from "@lucide/svelte";
	import type { ExchangeRate, ExchangeReport } from "../consumer";

	let { report, error }: { report: ExchangeReport | null; error: string | null } = $props();
	const pairs = ["USD", "GBP", "EUR"] as const;
	const rateFormatter = new Intl.NumberFormat("zh-CN", {
		minimumFractionDigits: 4,
		maximumFractionDigits: 4,
	});

	function find(code: string): ExchangeRate | null {
		if (report === null) return null;
		for (const rate of report.rates) {
			if (rate.code === code) return rate;
		}
		return null;
	}

	function pair(baseCode: string): { code: string; value: number; changePercent: number } | null {
		const base = find(baseCode);
		const cny = find("CNY");
		if (base === null || cny === null) return null;
		const value = cny.unitsPerEuro / base.unitsPerEuro;
		const previous = cny.previousUnitsPerEuro / base.previousUnitsPerEuro;
		return {
			code: `${baseCode}/CNY`,
			value,
			changePercent: previous === 0 ? 0 : ((value - previous) / previous) * 100,
		};
	}

	let rows = $derived(pairs.map(pair).filter((value): value is NonNullable<typeof value> => value !== null));
	let date = $derived.by(() => {
		const cny = find("CNY");
		return cny === null ? null : cny.date;
	});
</script>

<section class="exchange-panel" aria-label="Major exchange rates">
	<header><span><ArrowLeftRight size={15} /> Major Rates</span><small>ECB{date === null ? "" : ` · ${date}`}</small></header>
	{#if report !== null && rows.length === pairs.length}
		<div class="rates">
			{#each rows as row (row.code)}
				<div class:primary={row.code === "USD/CNY"}>
					<span>{row.code}</span>
					<strong>{rateFormatter.format(row.value)}</strong>
					<small class:negative={row.changePercent < 0}>
						{#if row.changePercent < 0}<TrendingDown size={11} />{:else}<TrendingUp size={11} />{/if}
						{row.changePercent >= 0 ? "+" : ""}{row.changePercent.toFixed(2)}%
					</small>
				</div>
			{/each}
		</div>
		<p>CNY reference value for one unit of foreign currency</p>
	{:else if error !== null}
		<p class="message" role="alert">{error}</p>
	{:else}
		<p class="message">Loading ECB exchange rates…</p>
	{/if}
</section>

<style>
	.exchange-panel { width: 100%; min-width: 0; box-sizing: border-box; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header,
	header span,
	.rates > div,
	.rates small { display: flex; align-items: center; }
	header { justify-content: space-between; color: var(--color-muted-foreground); }
	header span { gap: 0.4rem; font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
	header small { color: var(--color-accent); font-family: var(--font-mono); font-size: 0.5rem; }
	.rates { display: grid; gap: 0.05rem; margin-top: 0.7rem; }
	.rates > div { min-width: 0; gap: 0.6rem; padding: 0.45rem 0; border-bottom: 1px solid color-mix(in srgb, var(--color-border) 65%, transparent); }
	.rates > div:last-child { border-bottom: 0; }
	.rates span { width: 4.3rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.58rem; }
	.rates strong { flex: 1; color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.76rem; font-weight: 500; }
	.rates .primary strong { color: var(--color-accent); font-size: 0.95rem; }
	.rates small { gap: 0.15rem; color: var(--color-success); font-family: var(--font-mono); font-size: 0.5rem; }
	.rates small.negative { color: var(--color-error); }
	p { margin: 0.55rem 0 0; color: var(--color-muted-foreground); font-size: 0.5rem; }
	p.message { display: flex; min-height: 6.5rem; align-items: center; justify-content: center; font-size: 0.68rem; line-height: 1.4; text-align: center; }
</style>
