<script lang="ts">
	import type { QueryState, CherryInBalance, ClaudeUsage, CodexUsage, CopilotQuota, CopilotUsage, DeepSeekBalance, DimAgentUsage, GrokUsage, OpenCodeUsage, RateLimitWindow, TokenFluxUsage } from "../../consumer";

	type RateWindow = { label: string; usedPercent: number; resetsAt: number | string | null };
	type RateSection = { title: string; planType: string | null; windows: RateWindow[]; hasData: boolean; error: string | null };

	let source:
		| { provider: "codex"; state: QueryState<CodexUsage> }
		| { provider: "openCode"; state: QueryState<OpenCodeUsage> }
		| { provider: "claude"; state: QueryState<ClaudeUsage> }
		| { provider: "codexClaude"; codex: QueryState<CodexUsage>; claude: QueryState<ClaudeUsage> }
		| { provider: "grok"; state: QueryState<GrokUsage> }
		| { provider: "copilot"; state: QueryState<CopilotUsage> }
		| { provider: "deepSeek"; state: QueryState<DeepSeekBalance> }
		| { provider: "cherryIn"; state: QueryState<CherryInBalance> }
		| { provider: "tokenFlux"; state: QueryState<TokenFluxUsage> }
		| { provider: "dimAgent"; state: QueryState<DimAgentUsage> } = $props();

	const percentFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	const creditFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });
	const usdFormatter = new Intl.NumberFormat("en-US", { currency: "USD", currencyDisplay: "narrowSymbol", maximumFractionDigits: 2, minimumFractionDigits: 2, style: "currency" });
	let panelLabel = $derived(providerLabel(source.provider));
	let openCodeWindows = $derived(source.provider !== "openCode" || source.state.data === null ? [] : [
		{ label: "5 hours", window: source.state.data.usage.rolling, available: Math.max(0, 100 - source.state.data.usage.rolling.percent) },
		{ label: "Weekly", window: source.state.data.usage.weekly, available: Math.max(0, 100 - source.state.data.usage.weekly.percent) },
		{ label: "Monthly", window: source.state.data.usage.monthly, available: Math.max(0, 100 - source.state.data.usage.monthly.percent) },
	]);
	let copilotQuotas = $derived.by((): Array<{ label: string; quota: CopilotQuota }> => {
		if (source.provider !== "copilot" || source.state.data === null) return [];
		const copilot = source.state.data;
		const quotas: Array<{ label: string; quota: CopilotQuota | null }> = [
			{ label: "Premium requests", quota: copilot.quotaSnapshots.premiumInteractions },
			{ label: "Chat", quota: copilot.quotaSnapshots.chat },
			{ label: "Completions", quota: copilot.quotaSnapshots.completions },
		];
		return quotas.filter((item): item is { label: string; quota: CopilotQuota } => item.quota !== null && item.quota.unlimited !== true);
	});
	let rateSections = $derived.by((): RateSection[] => {
		if (source.provider === "codex") return [codexSection(source.state)];
		if (source.provider === "claude") return [claudeSection(source.state)];
		if (source.provider === "codexClaude") return [codexSection(source.codex), claudeSection(source.claude)];
		return [];
	});
	let tokenFluxMeters = $derived.by((): Array<{ label: string; percent: number; remaining: number; limit: number }> => {
		if (source.provider !== "tokenFlux" || source.state.data === null) return [];
		const subscription = source.state.data.subscription;
		const windows: Array<{ label: string; used: number; limit: number }> = [
			{ label: "Monthly", used: subscription.monthlyUsageUsd, limit: subscription.monthlyLimitUsd },
		];
		if (subscription.dailyLimitUsd !== null) {
			windows.unshift({ label: "Daily", used: subscription.dailyUsageUsd, limit: subscription.dailyLimitUsd });
		}
		if (subscription.weeklyLimitUsd !== null) {
			windows.push({ label: "Weekly", used: subscription.weeklyUsageUsd, limit: subscription.weeklyLimitUsd });
		}
		return windows
			.filter((window) => window.limit > 0)
			.map((window) => {
				const remaining = Math.max(0, window.limit - window.used);
				return {
					label: window.label,
					percent: Math.max(0, Math.min(100, remaining / window.limit * 100)),
					remaining,
					limit: window.limit,
				};
			});
	});
	let tokenFluxBalance = $derived.by(() => {
		if (source.provider !== "tokenFlux" || source.state.data === null) return null;
		const usage = source.state.data;
		return {
			remaining: usage.billing.remaining,
			unit: usage.billing.unit,
			planName: usage.billing.planName,
		};
	});
	let dimAgentMeter = $derived.by(() => {
		if (source.provider !== "dimAgent" || source.state.data === null) return null;
		const credits = source.state.data.credits;
		if (credits.totalUnits <= 0) return null;
		return {
			percent: Math.max(0, Math.min(100, credits.remainingUnits / credits.totalUnits * 100)),
			remaining: credits.remainingUnits,
			used: credits.usedUnits,
			total: credits.totalUnits,
			expiresAt: credits.expiresAt,
		};
	});
	let dimAgentFeatures = $derived.by((): Array<{ label: string; percent: number; used: number; allowance: number; unit: string }> => {
		if (source.provider !== "dimAgent" || source.state.data === null) return [];
		return source.state.data.featureMeters
			.filter((feature) => !feature.unlimited && feature.totalAllowance > 0)
			.map((feature) => ({
				label: feature.featureKey.replace(/_/g, " "),
				percent: Math.max(0, Math.min(100, feature.totalRemaining / feature.totalAllowance * 100)),
				used: feature.totalUsed,
				allowance: feature.totalAllowance,
				unit: feature.unit ?? "",
			}));
	});

	function remaining(window: { usedPercent: number }): number { return Math.max(0, Math.min(100, 100 - window.usedPercent)); }
	function providerLabel(value: typeof source.provider): string {
		if (value === "codex") return "Codex quota";
		if (value === "openCode") return "OpenCode Go quota";
		if (value === "claude") return "Claude quota";
		if (value === "codexClaude") return "Codex & Claude quota";
		if (value === "grok") return "Grok quota";
		if (value === "copilot") return "Copilot quota";
		if (value === "deepSeek") return "DeepSeek balance";
		if (value === "tokenFlux") return "TokenFlux usage";
		if (value === "dimAgent") return "DimAgent credits";
		return "Cherry balance";
	}
	function windowLabel(window: RateLimitWindow, fallback: string): string {
		if (window.windowDurationMins === 300) return "5 hours";
		if (window.windowDurationMins === 10_080) return "Weekly";
		if (window.windowDurationMins !== null) return `${window.windowDurationMins} minutes`;
		return fallback;
	}
	function codexSection(state: QueryState<CodexUsage>): RateSection {
		const codex = state.data;
		const windows: RateWindow[] = [];
		if (codex !== null) {
			if (codex.primary !== null) windows.push({ label: windowLabel(codex.primary, "Primary"), usedPercent: codex.primary.usedPercent, resetsAt: codex.primary.resetsAt });
			if (codex.secondary !== null) windows.push({ label: windowLabel(codex.secondary, "Secondary"), usedPercent: codex.secondary.usedPercent, resetsAt: codex.secondary.resetsAt });
		}
		return { title: "Codex", planType: codex?.planType ?? null, windows, hasData: codex !== null, error: state.error };
	}
	function claudeSection(state: QueryState<ClaudeUsage>): RateSection {
		const claude = state.data;
		const windows: RateWindow[] = [];
		if (claude !== null) {
			if (claude.fiveHour !== null) windows.push({ label: "5 hours", usedPercent: claude.fiveHour.usedPercent, resetsAt: claude.fiveHour.resetsAt });
			if (claude.sevenDay !== null) windows.push({ label: "Weekly", usedPercent: claude.sevenDay.usedPercent, resetsAt: claude.sevenDay.resetsAt });
		}
		return { title: "Claude", planType: claude?.planType ?? null, windows, hasData: claude !== null, error: state.error };
	}
	function resetLabel(timestamp: number | string | null): string {
		if (timestamp === null) return "Reset time unavailable";
		const date = new Date(typeof timestamp === "number" ? timestamp * 1000 : timestamp);
		if (Number.isNaN(date.getTime())) return "Reset time unavailable";
		return new Intl.DateTimeFormat("en-US", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
	}
	function loadingMessage(error: string | null): string { return error === null ? "Loading…" : error; }
	function copilotQuotaDetail(quota: CopilotQuota, accountReset: string | null): string {
		const resetAt = quota.quotaResetAt === null || quota.quotaResetAt === 0 ? accountReset : quota.quotaResetAt;
		const reset = `Resets ${resetLabel(resetAt)}`;
		if (quota.remaining === null || quota.entitlement === null) return reset;
		return `${quota.remaining} of ${quota.entitlement} remaining · ${reset}`;
	}
</script>

{#snippet providerSection(section: RateSection)}
	<div class="provider-section">
		<div class="provider-heading"><strong>{section.title}</strong>{#if section.planType !== null}<span>{section.planType}</span>{/if}</div>
		{#if section.windows.length > 0}
			<div class="meter-list">{#each section.windows as window (window.label)}<div class="meter"><div><span>{window.label}</span><strong>{percentFormatter.format(remaining(window))}%</strong></div><div class="progress" role="progressbar" aria-label={`${section.title} ${window.label} quota`} aria-valuenow={remaining(window)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(window)}%`}></span></div><small>{resetLabel(window.resetsAt)}</small></div>{/each}</div>
		{:else if section.hasData}
			<p>No metered quota is available.</p>
		{:else if section.error === null}
			<p>Loading…</p>
		{/if}
		{#if section.error !== null}<p role="alert">{section.error}</p>{/if}
	</div>
{/snippet}

<section class="usage-panel" aria-label={panelLabel}>
	<article>
		{#if source.provider === "codex" || source.provider === "claude" || source.provider === "codexClaude"}
			{#each rateSections as section (section.title)}
				{@render providerSection(section)}
			{/each}
		{:else if source.provider === "openCode"}
			{@const openCode = source.state.data}
			<div class="provider-heading"><strong>OpenCode Go</strong><span>Go</span></div>
			{#if openCode !== null}<div class="meter-list">{#each openCodeWindows as item (item.label)}<div class="meter"><div><span>{item.label}</span><strong>{percentFormatter.format(item.available)}%</strong></div><div class:limited={item.window.status === "rate-limited"} class="progress" role="progressbar" aria-label={`OpenCode Go ${item.label} quota`} aria-valuenow={item.available} aria-valuemin="0" aria-valuemax="100"><span style:width={`${item.available}%`}></span></div><small>{resetLabel(item.window.resetsAt)}</small></div>{/each}</div>{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{:else if source.provider === "grok"}
			{@const grok = source.state.data}
			<div class="provider-heading"><strong>Grok</strong>{#if grok !== null && grok.planType !== null}<span>{grok.planType}</span>{/if}</div>
			{#if grok !== null}<div class="meter-list"><div class="meter"><div><span>{grok.window.windowDurationMins === 10_080 ? "Weekly" : "Current period"}</span><strong>{percentFormatter.format(remaining(grok.window))}%</strong></div><div class="progress" role="progressbar" aria-label="Grok quota" aria-valuenow={remaining(grok.window)} aria-valuemin="0" aria-valuemax="100"><span style:width={`${remaining(grok.window)}%`}></span></div><small>{resetLabel(grok.window.resetsAt)}</small></div></div>{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{:else if source.provider === "copilot"}
			{@const copilot = source.state.data}
			<div class="provider-heading"><strong>Copilot</strong>{#if copilot !== null && copilot.copilotPlan !== null}<span>{copilot.copilotPlan}</span>{/if}</div>
			{#if copilot !== null && copilotQuotas.length > 0}<div class="meter-list">{#each copilotQuotas as item (item.label)}{@const available = Math.max(0, Math.min(100, item.quota.percentRemaining === null ? 0 : item.quota.percentRemaining))}<div class="meter"><div><span>{item.label}</span><strong>{percentFormatter.format(available)}%</strong></div><div class="progress" role="progressbar" aria-label={`Copilot ${item.label} quota`} aria-valuenow={available} aria-valuemin="0" aria-valuemax="100"><span style:width={`${available}%`}></span></div><small>{copilotQuotaDetail(item.quota, copilot.quotaResetDateUtc)}</small></div>{/each}</div>{:else if copilot !== null}<p>No metered Copilot quota is available.</p>{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{:else if source.provider === "deepSeek"}
			{@const deepSeek = source.state.data}
			<div class="provider-heading"><strong>DeepSeek</strong>{#if deepSeek !== null}<span class:unavailable={!deepSeek.isAvailable}>{deepSeek.isAvailable ? "Available" : "Unavailable"}</span>{/if}</div>
			{#if deepSeek !== null && deepSeek.balanceInfos.length > 0}<div class="account-balances">{#each deepSeek.balanceInfos as balance (balance.currency)}<div class="account-balance"><div><strong>{balance.currency === "CNY" ? "¥" : "$"}{balance.totalBalance}</strong><span>{balance.currency === "CNY" ? "RMB" : balance.currency}</span></div><small>Available balance</small></div>{/each}</div>{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{:else if source.provider === "tokenFlux"}
			{@const tokenFlux = source.state.data}
			{@const meters = tokenFluxMeters}
			{@const balance = tokenFluxBalance}
			<div class="provider-heading"><strong>TokenFlux</strong>{#if balance !== null}<span>{balance.planName}</span>{/if}</div>
			{#if tokenFlux !== null}
				{#if source.state.error !== null}<p role="alert">{source.state.error} Showing last successful data.</p>{/if}
				{#if meters.length > 0 || tokenFlux.subscription.dailyLimitUsd === null}<div class="meter-list">{#if tokenFlux.subscription.dailyLimitUsd === null}<div class="meter" aria-label="TokenFlux daily usage"><div><span>Daily used</span><strong>{usdFormatter.format(tokenFlux.subscription.dailyUsageUsd)} USD</strong></div><small>Daily limit not provided</small></div>{/if}{#each meters as meter (meter.label)}<div class="meter"><div><span>{meter.label}</span><strong>{percentFormatter.format(meter.percent)}%</strong></div><div class="progress" role="progressbar" aria-label={`TokenFlux ${meter.label} quota`} aria-valuenow={meter.percent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${meter.percent}%`}></span></div><small>{usdFormatter.format(meter.remaining)} / {usdFormatter.format(meter.limit)} USD remaining</small></div>{/each}</div>{/if}
				{#if balance !== null}<div class="account-balances"><div class="account-balance"><div><strong>{creditFormatter.format(balance.remaining)}</strong><span>{balance.unit}</span></div><small>Available balance</small></div></div>{/if}
			{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{:else if source.provider === "dimAgent"}
			{@const dimAgent = source.state.data}
			{@const meter = dimAgentMeter}
			{@const features = dimAgentFeatures}
			<div class="provider-heading"><strong>DimAgent</strong>{#if dimAgent !== null && dimAgent.planName !== null}<span>{dimAgent.planName}</span>{/if}</div>
			{#if dimAgent !== null}
				{#if source.state.error !== null}<p role="alert">{source.state.error}</p>{/if}
				<div class="meter-list">
					{#if meter !== null}
					<div class="meter"><div><span>Credits</span><strong>{percentFormatter.format(meter.percent)}%</strong></div><div class="progress" role="progressbar" aria-label="DimAgent credits" aria-valuenow={meter.percent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${meter.percent}%`}></span></div><small>{creditFormatter.format(meter.used)} / {creditFormatter.format(meter.total)} used{#if meter.expiresAt !== null} · resets {resetLabel(meter.expiresAt)}{/if}</small></div>
					{/if}
					{#each features as feature (feature.label)}<div class="meter"><div><span>{feature.label}</span><strong>{percentFormatter.format(feature.percent)}%</strong></div><div class="progress" role="progressbar" aria-label={`DimAgent ${feature.label}`} aria-valuenow={feature.percent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${feature.percent}%`}></span></div><small>{creditFormatter.format(feature.used)} / {creditFormatter.format(feature.allowance)} {feature.unit} used</small></div>{/each}
					{#if meter === null && features.length === 0}<p>No metered quota is available.</p>{/if}
				</div>
			{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{:else}
			{@const cherryIn = source.state.data}
			<div class="provider-heading"><strong>Cherry</strong>{#if cherryIn !== null}<span>Available</span>{/if}</div>
			{#if cherryIn !== null}<div class="account-balances"><div class="account-balance"><div><strong>{usdFormatter.format(cherryIn.balance)}</strong><span>USD</span></div><small>Available balance</small></div></div>{:else}<p>{loadingMessage(source.state.error)}</p>{/if}
		{/if}
	</article>
</section>

<style>
	.usage-panel { display: grid; min-width: 0; min-height: 7.5rem; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	article { display: grid; min-width: 0; height: 100%; align-content: center; box-sizing: border-box; padding: 0.75rem; }
	.provider-heading,
	.provider-heading strong,
	.meter > div { display: flex; align-items: center; }
	.provider-heading { min-height: 1.2rem; justify-content: space-between; gap: 0.4rem; margin-bottom: 0.4rem; }
	.provider-section + .provider-section { margin-top: 0.75rem; padding-top: 0.75rem; border-top: 1px solid var(--color-border); }
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
	.progress.limited span { background: var(--color-error); }
	small,
	p { margin: 0; color: var(--color-muted-foreground); font-size: 0.55rem; line-height: 1.4; }
	p[role="alert"] { margin-top: 0.4rem; color: var(--color-error); }
	.account-balances { grid-template-columns: repeat(auto-fit, minmax(min(100%, 7rem), 1fr)); gap: 0.4rem; }
	.account-balance { display: grid; min-width: 0; min-height: 3.4rem; align-content: center; gap: 0.25rem; padding: 0.55rem 0.65rem; border-radius: var(--radius-md); background: var(--color-muted); }
	.account-balance > div { display: flex; align-items: baseline; justify-content: space-between; gap: 0.4rem; }
	.account-balance strong { overflow: hidden; font-family: var(--font-mono); font-size: 0.9rem; font-weight: 500; text-overflow: ellipsis; }
	.account-balance span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.52rem; }
	.account-balance small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	@media (prefers-reduced-motion: reduce) { .progress span { transition: none; } }
</style>
