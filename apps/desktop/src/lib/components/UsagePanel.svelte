<script lang="ts">
	import type { CherryInBalance, ClaudeUsage, CodexUsage, CopilotQuota, CopilotUsage, DeepSeekBalance, GrokUsage, OpenCodeUsage, RateLimitWindow } from "../consumer";

	let { provider, codex, codexError, openCode, openCodeError, claude, claudeError, grok, grokError, copilot, copilotError, deepSeek, deepSeekError, cherryIn, cherryInError }: {
		provider: "codex" | "openCode" | "claude" | "grok" | "copilot" | "deepSeek" | "cherryIn";
		codex: CodexUsage | null;
		codexError: string | null;
		openCode: OpenCodeUsage | null;
		openCodeError: string | null;
		claude: ClaudeUsage | null;
		claudeError: string | null;
		grok: GrokUsage | null;
		grokError: string | null;
		copilot: CopilotUsage | null;
		copilotError: string | null;
		deepSeek: DeepSeekBalance | null;
		deepSeekError: string | null;
		cherryIn: CherryInBalance | null;
		cherryInError: string | null;
	} = $props();

	const percentFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	const usdFormatter = new Intl.NumberFormat("en-US", { currency: "USD", currencyDisplay: "narrowSymbol", maximumFractionDigits: 2, minimumFractionDigits: 2, style: "currency" });
	let panelLabel = $derived(providerLabel(provider));
	let openCodeWindows = $derived(openCode === null ? [] : [
		{ label: "5 hours", window: openCode.usage.rolling, available: Math.max(0, 100 - openCode.usage.rolling.percent) },
		{ label: "Weekly", window: openCode.usage.weekly, available: Math.max(0, 100 - openCode.usage.weekly.percent) },
		{ label: "Monthly", window: openCode.usage.monthly, available: Math.max(0, 100 - openCode.usage.monthly.percent) },
	]);
	let copilotQuotas = $derived.by((): Array<{ label: string; quota: CopilotQuota }> => {
		if (copilot === null) return [];
		const quotas: Array<{ label: string; quota: CopilotQuota | null }> = [
			{ label: "Premium requests", quota: copilot.quotaSnapshots.premiumInteractions },
			{ label: "Chat", quota: copilot.quotaSnapshots.chat },
			{ label: "Completions", quota: copilot.quotaSnapshots.completions },
		];
		return quotas.filter((item): item is { label: string; quota: CopilotQuota } => item.quota !== null && item.quota.unlimited !== true);
	});

	function remaining(window: { usedPercent: number }): number { return Math.max(0, Math.min(100, 100 - window.usedPercent)); }
	function providerLabel(value: typeof provider): string {
		if (value === "codex") return "Codex quota";
		if (value === "openCode") return "OpenCode Go quota";
		if (value === "claude") return "Claude quota";
		if (value === "grok") return "Grok quota";
		if (value === "copilot") return "Copilot quota";
		if (value === "deepSeek") return "DeepSeek balance";
		return "Cherry balance";
	}
	function windowLabel(window: RateLimitWindow, fallback: string): string {
		if (window.windowDurationMins === 300) return "5 hours";
		if (window.windowDurationMins === 10_080) return "Weekly";
		if (window.windowDurationMins !== null) return `${window.windowDurationMins} minutes`;
		return fallback;
	}
	function resetLabel(timestamp: number | string | null): string {
		if (timestamp === null) return "Reset time unavailable";
		const date = new Date(typeof timestamp === "number" ? timestamp * 1000 : timestamp);
		if (Number.isNaN(date.getTime())) return "Reset time unavailable";
		return new Intl.DateTimeFormat("en-US", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
	}
	function loadingMessage(error: string | null): string { return error === null ? "Loading…" : error; }
	function copilotResetTimestamp(quota: CopilotQuota, accountReset: string | null): number | string | null {
		if (quota.quotaResetAt === null || quota.quotaResetAt === 0) return accountReset;
		return quota.quotaResetAt;
	}
	function copilotQuotaDetail(quota: CopilotQuota, accountReset: string | null): string {
		const reset = `Resets ${resetLabel(copilotResetTimestamp(quota, accountReset))}`;
		if (quota.remaining === null || quota.entitlement === null) return reset;
		return `${quota.remaining} of ${quota.entitlement} remaining · ${reset}`;
	}
</script>

<section class="usage-panel" aria-label={panelLabel}>
	<article>
		{#if provider === "codex"}
			<div class="provider-heading"><strong>Codex</strong>{#if codex !== null && codex.planType !== null}<span>{codex.planType}</span>{/if}</div>
			{#if codex !== null}
				<div class="meter-list">
					{#if codex.primary !== null}<div class="meter"><div><span>{windowLabel(codex.primary, "Primary")}</span><strong>{percentFormatter.format(remaining(codex.primary))}%</strong></div><div class="progress" role="progressbar" aria-label="Codex primary quota" aria-valuenow={remaining(codex.primary)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(codex.primary)}%`}></span></div><small>{resetLabel(codex.primary.resetsAt)}</small></div>{/if}
					{#if codex.secondary !== null}<div class="meter"><div><span>{windowLabel(codex.secondary, "Secondary")}</span><strong>{percentFormatter.format(remaining(codex.secondary))}%</strong></div><div class="progress" role="progressbar" aria-label="Codex secondary quota" aria-valuenow={remaining(codex.secondary)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(codex.secondary)}%`}></span></div><small>{resetLabel(codex.secondary.resetsAt)}</small></div>{/if}
					{#if codex.spark !== null && codex.spark.primary !== null}<div class="meter"><div><span>GPT-5.3 Codex Spark</span><strong>{percentFormatter.format(remaining(codex.spark.primary))}%</strong></div><div class="progress spark" role="progressbar" aria-label="GPT-5.3 Codex Spark quota" aria-valuenow={remaining(codex.spark.primary)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(codex.spark.primary)}%`}></span></div><small>{resetLabel(codex.spark.primary.resetsAt)}</small></div>{/if}
				</div>
			{:else}<p>{loadingMessage(codexError)}</p>{/if}
		{:else if provider === "openCode"}
			<div class="provider-heading"><strong>OpenCode Go</strong><span>Go</span></div>
			{#if openCode !== null}<div class="meter-list">{#each openCodeWindows as item}<div class="meter"><div><span>{item.label}</span><strong>{percentFormatter.format(item.available)}%</strong></div><div class:limited={item.window.status === "rate-limited"} class="progress" role="progressbar" aria-label={`OpenCode Go ${item.label} quota`} aria-valuenow={item.available} aria-valuemin="0" aria-valuemax="100"><span style:width={`${item.available}%`}></span></div><small>{resetLabel(item.window.resetsAt)}</small></div>{/each}</div>{:else}<p>{loadingMessage(openCodeError)}</p>{/if}
		{:else if provider === "claude"}
			<div class="provider-heading"><strong>Claude</strong>{#if claude !== null}<span>{claude.planType}</span>{/if}</div>
			{#if claude !== null}<div class="meter-list">{#if claude.fiveHour !== null}<div class="meter"><div><span>5 hours</span><strong>{percentFormatter.format(remaining(claude.fiveHour))}%</strong></div><div class="progress" role="progressbar" aria-label="Claude 5-hour quota" aria-valuenow={remaining(claude.fiveHour)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(claude.fiveHour)}%`}></span></div><small>{resetLabel(claude.fiveHour.resetsAt)}</small></div>{/if}{#if claude.sevenDay !== null}<div class="meter"><div><span>Weekly</span><strong>{percentFormatter.format(remaining(claude.sevenDay))}%</strong></div><div class="progress" role="progressbar" aria-label="Claude weekly quota" aria-valuenow={remaining(claude.sevenDay)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(claude.sevenDay)}%`}></span></div><small>{resetLabel(claude.sevenDay.resetsAt)}</small></div>{/if}</div>{:else}<p>{loadingMessage(claudeError)}</p>{/if}
		{:else if provider === "grok"}
			<div class="provider-heading"><strong>Grok</strong>{#if grok !== null && grok.planType !== null}<span>{grok.planType}</span>{/if}</div>
			{#if grok !== null}<div class="meter-list"><div class="meter"><div><span>{grok.window.windowDurationMins === 10_080 ? "Weekly" : "Current period"}</span><strong>{percentFormatter.format(remaining(grok.window))}%</strong></div><div class="progress" role="progressbar" aria-label="Grok quota" aria-valuenow={remaining(grok.window)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(grok.window)}%`}></span></div><small>{resetLabel(grok.window.resetsAt)}</small></div></div>{:else}<p>{loadingMessage(grokError)}</p>{/if}
		{:else if provider === "copilot"}
			<div class="provider-heading"><strong>Copilot</strong>{#if copilot !== null && copilot.copilotPlan !== null}<span>{copilot.copilotPlan}</span>{/if}</div>
			{#if copilot !== null && copilotQuotas.length > 0}<div class="meter-list">{#each copilotQuotas as item}{@const available = Math.max(0, Math.min(100, item.quota.percentRemaining === null ? 0 : item.quota.percentRemaining))}<div class="meter"><div><span>{item.label}</span><strong>{percentFormatter.format(available)}%</strong></div><div class="progress" role="progressbar" aria-label={`Copilot ${item.label} quota`} aria-valuenow={available} aria-valuemin="0" aria-valuemax="100"><span style:width={`${available}%`}></span></div><small>{copilotQuotaDetail(item.quota, copilot.quotaResetDateUtc)}</small></div>{/each}</div>{:else if copilot !== null}<p>No metered Copilot quota is available.</p>{:else}<p>{loadingMessage(copilotError)}</p>{/if}
		{:else if provider === "deepSeek"}
			<div class="provider-heading"><strong>DeepSeek</strong>{#if deepSeek !== null}<span class:unavailable={!deepSeek.isAvailable}>{deepSeek.isAvailable ? "Available" : "Unavailable"}</span>{/if}</div>
			{#if deepSeek !== null && deepSeek.balanceInfos.length > 0}<div class="account-balances">{#each deepSeek.balanceInfos as balance}<div class="account-balance"><div><strong>{balance.currency === "CNY" ? "¥" : "$"}{balance.totalBalance}</strong><span>{balance.currency === "CNY" ? "RMB" : balance.currency}</span></div><small>Available balance</small></div>{/each}</div>{:else}<p>{loadingMessage(deepSeekError)}</p>{/if}
		{:else}
			<div class="provider-heading"><strong>Cherry</strong>{#if cherryIn !== null}<span>Available</span>{/if}</div>
			{#if cherryIn !== null}<div class="account-balances"><div class="account-balance"><div><strong>{usdFormatter.format(cherryIn.balance)}</strong><span>USD</span></div><small>Available balance</small></div></div>{:else}<p>{loadingMessage(cherryInError)}</p>{/if}
		{/if}
	</article>
</section>

<style>
	.usage-panel { display: grid; min-width: 0; min-height: 8.5rem; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	article { display: grid; min-width: 0; height: 100%; align-content: center; box-sizing: border-box; padding: 0.85rem; }
	.provider-heading,
	.provider-heading strong,
	.meter > div { display: flex; align-items: center; }
	.provider-heading { min-height: 1.2rem; justify-content: space-between; gap: 0.4rem; margin-bottom: 0.55rem; }
	.provider-heading strong { gap: 0.25rem; font-size: 0.68rem; }
	.provider-heading > span { padding: 0.1rem 0.3rem; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-muted-foreground); font-size: 0.55rem; text-transform: uppercase; }
	.provider-heading > span.unavailable { color: var(--color-error); }
	.meter-list,
	.account-balances { display: grid; }
	.meter-list { gap: 0.5rem; }
	.meter { min-width: 0; }
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
	@media (prefers-reduced-motion: reduce) { .progress span { transition: none; } }
</style>
