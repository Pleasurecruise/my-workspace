import { afterEach, expect, it } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import UsagePanel from "../UsagePanel.svelte";

const target = document.createElement("div");
let view: ReturnType<typeof mount> | null = null;

afterEach(async () => {
	if (view !== null) await unmount(view);
	view = null;
	target.replaceChildren();
});

it("preserves independent provider failures and settled quotas in the combined card", async () => {
	view = mount(UsagePanel, {
		target,
		props: {
			provider: "codexClaude",
			codex: { data: null, error: "Codex timed out", loading: false },
			claude: {
				data: {
					planType: "Pro",
					fiveHour: { usedPercent: 20, windowDurationMins: 300, resetsAt: null },
					sevenDay: null,
				},
				error: "Claude refresh failed",
				loading: false,
			},
		},
	});
	await tick();
	expect(target.textContent).toContain("Codex timed out");
	expect(target.textContent).toContain("Claude refresh failed");
	expect(target.textContent).toContain("80%");
	expect(target.textContent).not.toContain("Not signed in");
	expect(target.querySelectorAll('[role="alert"]')).toHaveLength(2);
});

it("shows DimAgent feature allowances when the account has no credit allowance", async () => {
	view = mount(UsagePanel, {
		target,
		props: {
			provider: "dimAgent",
			state: {
				data: {
					planName: null,
					credits: { totalUnits: 0, remainingUnits: 0, usedUnits: 0, expiresAt: null },
					featureMeters: [
						{
							featureKey: "web_search",
							totalRemaining: 50,
							totalAllowance: 100,
							totalUsed: 50,
							unlimited: false,
							unit: "calls",
							periodEnd: null,
						},
					],
				},
				error: "DimAgent refresh failed",
				loading: false,
			},
		},
	});
	await tick();
	expect(target.textContent).toContain("web search");
	expect(target.querySelector('[role="alert"]')?.textContent).toBe("DimAgent refresh failed");
	expect(target.textContent).toContain("50%");
	expect(target.textContent).not.toContain("Loading");
});

it("shows an explicit empty state for an unmetered DimAgent account", async () => {
	view = mount(UsagePanel, {
		target,
		props: {
			provider: "dimAgent",
			state: {
				data: {
					planName: null,
					credits: { totalUnits: 0, remainingUnits: 0, usedUnits: 0, expiresAt: null },
					featureMeters: [],
				},
				error: null,
				loading: false,
			},
		},
	});
	await tick();
	expect(target.textContent).toContain("No metered quota is available.");
	expect(target.textContent).not.toContain("Loading");
});

it.each([null, 100])("shows TokenFlux daily usage with daily limit %s", async (dailyLimitUsd) => {
	view = mount(UsagePanel, {
		target,
		props: {
			provider: "tokenFlux",
			state: {
				loading: false,
				error: "Refresh failed",
				data: {
					remaining: 80,
					unit: "credits",
					planName: "Lite",
					isValid: true,
					mode: "auto",
					billing: {
						available: true,
						mode: "auto",
						planId: 2,
						planName: "Lite",
						preferredSubscriptionId: null,
						remaining: 80,
						source: "subscription",
						subscriptionId: 1,
						unit: "credits",
					},
					subscription: {
						dailyLimitUsd,
						dailyUsageUsd: 10,
						expiresAt: null,
						id: 1,
						monthlyLimitUsd: 100,
						monthlyUsageUsd: 20,
						planId: 2,
						status: "active",
						weeklyLimitUsd: null,
						weeklyUsageUsd: 0,
						weeklyWindowStart: null,
					},
					usage: {
						averageDurationMs: 0,
						rpm: 0,
						tpm: 0,
						today: {
							actualCost: 0,
							cacheCreationTokens: 0,
							cacheReadTokens: 0,
							cost: 0,
							inputTokens: 0,
							outputTokens: 0,
							requests: 0,
							totalTokens: 0,
						},
						total: {
							actualCost: 0,
							cacheCreationTokens: 0,
							cacheReadTokens: 0,
							cost: 0,
							inputTokens: 0,
							outputTokens: 0,
							requests: 0,
							totalTokens: 0,
						},
					},
				},
			},
		},
	});
	await tick();
	if (dailyLimitUsd === null) {
		expect(target.querySelector('[aria-label="TokenFlux Daily quota"]')).toBeNull();
		expect(target.querySelector('[aria-label="TokenFlux daily usage"]')?.textContent).toContain(
			"10.00 USD",
		);
		expect(target.textContent).toContain("Daily limit not provided");
	} else {
		expect(
			target.querySelector('[aria-label="TokenFlux Daily quota"]')?.getAttribute("aria-valuenow"),
		).toBe("90");
		expect(target.textContent).not.toContain("Daily limit not provided");
	}
	expect(
		target.querySelector('[aria-label="TokenFlux Monthly quota"]')?.getAttribute("aria-valuenow"),
	).toBe("80");
	expect(target.textContent).toContain("Available balance");
	expect(target.textContent).toContain("Showing last successful data.");
});
