<script lang="ts">
	import { Gauge, WalletCards } from "@lucide/svelte";
	import type { CherryInBalance, CodexUsage, DeepSeekBalance, OpenCodeUsage, RateLimitWindow } from "../consumer";

	let { codex, codexError, openCode, openCodeError, deepSeek, deepSeekError, cherryIn, cherryInError }: {
		codex: CodexUsage | null;
		codexError: string | null;
		openCode: OpenCodeUsage | null;
		openCodeError: string | null;
		deepSeek: DeepSeekBalance | null;
		deepSeekError: string | null;
		cherryIn: CherryInBalance | null;
		cherryInError: string | null;
	} = $props();

	const percentFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	const usdFormatter = new Intl.NumberFormat("en-US", {
		currency: "USD",
		currencyDisplay: "narrowSymbol",
		maximumFractionDigits: 2,
		minimumFractionDigits: 2,
		style: "currency",
	});

	let openCodeWindows = $derived.by(() => {
		if (openCode === null) return [];
		return [
			{ label: "5 小时", window: openCode.usage.rolling, available: Math.max(0, 100 - openCode.usage.rolling.percent) },
			{ label: "每周", window: openCode.usage.weekly, available: Math.max(0, 100 - openCode.usage.weekly.percent) },
			{ label: "每月", window: openCode.usage.monthly, available: Math.max(0, 100 - openCode.usage.monthly.percent) },
		];
	});

	function remaining(window: RateLimitWindow): number {
		return Math.max(0, Math.min(100, 100 - window.usedPercent));
	}

	function windowLabel(window: RateLimitWindow, fallback: string): string {
		if (window.windowDurationMins === 300) return "5 小时";
		if (window.windowDurationMins === 10_080) return "每周";
		if (window.windowDurationMins !== null) return `${window.windowDurationMins} 分钟`;
		return fallback;
	}

	function resetLabel(timestamp: number | string | null): string {
		if (timestamp === null) return "重置时间未知";
		const date = new Date(typeof timestamp === "number" ? timestamp * 1000 : timestamp);
		if (Number.isNaN(date.getTime())) return "重置时间未知";
		return new Intl.DateTimeFormat("zh-CN", {
			month: "numeric",
			day: "numeric",
			hour: "2-digit",
			minute: "2-digit",
		}).format(date);
	}
</script>

<section class="usage-panel" aria-label="额度与余额">
	<div class="usage-row">
		<div class="row-label"><Gauge size={14} /><span>额度</span></div>
		<div class="quota-grid">
			<article>
				<div class="provider-heading"><strong>Codex</strong>{#if codex !== null && codex.planType !== null}<span>{codex.planType}</span>{/if}</div>
				{#if codex !== null}
					<div class="meter-list">
						{#if codex.primary !== null}
							<div class="meter">
								<div><span>{windowLabel(codex.primary, "主要")}</span><strong>{percentFormatter.format(remaining(codex.primary))}%</strong></div>
								<div class="progress" role="progressbar" aria-label="Codex 主要额度" aria-valuenow={remaining(codex.primary)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(codex.primary)}%`}></span></div>
								<small>{resetLabel(codex.primary.resetsAt)}</small>
							</div>
						{/if}
						{#if codex.secondary !== null}
							<div class="meter">
								<div><span>{windowLabel(codex.secondary, "次要")}</span><strong>{percentFormatter.format(remaining(codex.secondary))}%</strong></div>
								<div class="progress" role="progressbar" aria-label="Codex 次要额度" aria-valuenow={remaining(codex.secondary)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(codex.secondary)}%`}></span></div>
								<small>{resetLabel(codex.secondary.resetsAt)}</small>
							</div>
						{/if}
						{#if codex.spark !== null && codex.spark.primary !== null}
							<div class="meter">
								<div><span>GPT-5.3 Codex Spark</span><strong>{percentFormatter.format(remaining(codex.spark.primary))}%</strong></div>
								<div class="progress spark" role="progressbar" aria-label="GPT-5.3 Codex Spark 额度" aria-valuenow={remaining(codex.spark.primary)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(codex.spark.primary)}%`}></span></div>
								<small>{resetLabel(codex.spark.primary.resetsAt)}</small>
							</div>
						{/if}
					</div>
				{:else}
					{#if codexError !== null}<p>{codexError}</p>{:else}<p>正在读取…</p>{/if}
				{/if}
			</article>

			<article>
				<div class="provider-heading"><strong>OpenCode Go</strong><span>Go</span></div>
				{#if openCode !== null}
					<div class="meter-list">
						{#each openCodeWindows as item}
							<div class="meter">
								<div><span>{item.label}</span><strong>{percentFormatter.format(item.available)}%</strong></div>
								<div class:limited={item.window.status === "rate-limited"} class="progress" role="progressbar" aria-label={`OpenCode Go ${item.label}额度`} aria-valuenow={item.available} aria-valuemin="0" aria-valuemax="100"><span style:width={`${item.available}%`}></span></div>
								<small>{resetLabel(item.window.resetsAt)}</small>
							</div>
						{/each}
					</div>
				{:else}
					{#if openCodeError !== null}<p>{openCodeError}</p>{:else}<p>正在读取…</p>{/if}
				{/if}
			</article>
		</div>
	</div>

	<div class="usage-row">
		<div class="row-label"><WalletCards size={14} /><span>余额</span></div>
		<div class="balance-grid">
			<article>
				<div class="provider-heading"><strong>DeepSeek</strong>{#if deepSeek !== null}<span class:unavailable={!deepSeek.isAvailable}>{deepSeek.isAvailable ? "可用" : "不可用"}</span>{/if}</div>
				{#if deepSeek !== null && deepSeek.balanceInfos.length > 0}
					<div class="account-balances">
						{#each deepSeek.balanceInfos as balance}
							<div class="account-balance">
								<div><strong>{balance.currency === "CNY" ? "¥" : "$"}{balance.totalBalance}</strong><span>{balance.currency === "CNY" ? "RMB" : balance.currency}</span></div>
								<small>账户可用余额</small>
							</div>
						{/each}
					</div>
				{:else}
					{#if deepSeekError !== null}<p>{deepSeekError}</p>{:else}<p>正在读取…</p>{/if}
				{/if}
			</article>

			<article>
				<div class="provider-heading"><strong>Cherry</strong>{#if cherryIn !== null}<span>可用</span>{/if}</div>
				{#if cherryIn !== null}
					<div class="account-balances">
						<div class="account-balance">
							<div><strong>{usdFormatter.format(cherryIn.balance)}</strong><span>USD</span></div>
							<small>账户可用余额</small>
						</div>
					</div>
				{:else}
					{#if cherryInError !== null}<p>{cherryInError}</p>{:else}<p>正在读取…</p>{/if}
				{/if}
			</article>
		</div>
	</div>
</section>

<style>
	.usage-panel {
		display: grid;
		grid-template-rows: minmax(0, 1fr) auto;
		min-width: 0;
		overflow: hidden;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
	}

	.usage-row {
		display: grid;
		grid-template-columns: 3.25rem minmax(0, 1fr);
	}

	.usage-row + .usage-row {
		border-top: 1px solid var(--color-divider);
	}

	.row-label {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.3rem;
		border-right: 1px solid var(--color-divider);
		color: var(--color-muted-foreground);
		font-size: 0.65rem;
		font-weight: 600;
		writing-mode: vertical-rl;
	}

	.quota-grid,
	.balance-grid {
		display: grid;
		min-width: 0;
	}

	.quota-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
	.balance-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }

	article {
		min-width: 0;
		padding: 0.7rem 0.75rem;
	}

	article + article { border-left: 1px solid var(--color-divider); }

	.provider-heading,
	.provider-heading strong,
	.meter > div {
		display: flex;
		align-items: center;
	}

	.provider-heading {
		min-height: 1.2rem;
		justify-content: space-between;
		gap: 0.4rem;
		margin-bottom: 0.55rem;
	}

	.provider-heading strong { gap: 0.25rem; font-size: 0.68rem; }
	.provider-heading > span { padding: 0.1rem 0.3rem; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-muted-foreground); font-size: 0.55rem; text-transform: uppercase; }
	.provider-heading > span.unavailable { color: var(--color-error); }

	.meter-list,
	.account-balances { display: grid; }
	.meter-list { gap: 0.5rem; }
	.meter { gap: 0.2rem; min-width: 0; }
	.meter > div { justify-content: space-between; gap: 0.35rem; }
	.meter span,
	.meter strong { font-size: 0.6rem; }
	.meter strong { font-family: var(--font-mono); }

	.progress { height: 0.28rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.progress span { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); transition: width var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.progress.spark span { background: var(--color-warning); }
	.progress.limited span { background: var(--color-error); }

	small,
	p { margin: 0; color: var(--color-muted-foreground); font-size: 0.55rem; line-height: 1.4; }
	.account-balances { grid-template-columns: repeat(auto-fit, minmax(min(100%, 7rem), 1fr)); gap: 0.4rem; }
	.account-balance { display: grid; min-width: 0; min-height: 3.4rem; align-content: center; gap: 0.25rem; padding: 0.55rem 0.65rem; border-radius: var(--radius-md); background: var(--color-muted); }
	.account-balance > div { display: flex; align-items: baseline; justify-content: space-between; gap: 0.4rem; }
	.account-balance strong { overflow: hidden; font-family: var(--font-mono); font-size: 0.9rem; font-weight: 500; text-overflow: ellipsis; }
	.account-balance span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.52rem; }
	.account-balance small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	@media (max-width: 620px) {
		.usage-row { grid-template-columns: 2.5rem minmax(0, 1fr); }
		.quota-grid,
		.balance-grid { grid-template-columns: 1fr; }
		article + article { border-top: 1px solid var(--color-divider); border-left: 0; }
	}

	@media (prefers-reduced-motion: reduce) {
		.progress span { animation: none; transition: none; }
	}
</style>
