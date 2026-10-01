import { afterEach, expect, it } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import ExchangePanel from "../ExchangePanel.svelte";
import QuotationPanel from "../QuotationPanel.svelte";
import ServiceStatusPanel from "../ServiceStatusPanel.svelte";
import StocksPanel from "../StocksPanel.svelte";
import WeatherPanel from "../WeatherPanel.svelte";
import type { ExchangeRate, StockReport } from "@/lib/contracts/dashboard";

const target = document.createElement("div");
let view: ReturnType<typeof mount> | null = null;

afterEach(async () => {
	if (view !== null) await unmount(view);
	view = null;
	target.replaceChildren();
});

function rate(code: string, unitsPerEuro: number, previousUnitsPerEuro: number): ExchangeRate {
	return {
		code,
		name: code,
		date: "2026-09-30",
		unitsPerEuro,
		previousUnitsPerEuro,
		change: 0,
		changePercent: 0,
	};
}

it("derives CNY cross rates and their daily change from euro references", async () => {
	view = mount(ExchangePanel, {
		target,
		props: {
			report: {
				referenceCurrency: "EUR",
				preparedAt: "2026-09-30T16:00:00Z",
				rates: [rate("CNY", 8, 8), rate("USD", 1, 1.1), rate("GBP", 0.8, 0.8), rate("EUR", 1, 1)],
			},
			error: null,
		},
	});
	await tick();
	const text = target.textContent.replaceAll(/\s+/g, " ");
	expect(text).toContain("ECB · 2026-09-30");
	expect(text).toContain("USD/CNY 8.0000 +10.00%");
	expect(text).toContain("GBP/CNY 10.0000 +0.00%");
	expect(text).toContain("EUR/CNY 8.0000 +0.00%");
});

it("shows the error instead of partial exchange rates", async () => {
	view = mount(ExchangePanel, {
		target,
		props: {
			report: { referenceCurrency: "EUR", preparedAt: "", rates: [rate("CNY", 8, 8)] },
			error: "ECB unavailable",
		},
	});
	await tick();
	expect(target.querySelector('[role="alert"]')?.textContent).toBe("ECB unavailable");
	expect(target.textContent).not.toContain("/CNY");
});

it("selects its own stock series and reports per-symbol failures", async () => {
	const stocks: StockReport = {
		stocks: [
			{
				symbol: "MSFT",
				name: "MSFT Inc.",
				currency: "USD",
				exchange: "NASDAQ",
				price: 110,
				change: -1.5,
				changePercent: -1.5,
				points: [
					{ timestamp: 1, close: 100 },
					{ timestamp: 2, close: 110 },
				],
			},
		],
		failures: [{ symbol: "AAPL", message: "AAPL quote unavailable" }],
	};
	view = mount(StocksPanel, { target, props: { stocks, symbol: "MSFT", error: null } });
	await tick();
	expect(target.textContent).toContain("MSFT Inc.");
	expect(target.textContent).toContain("-1.50%");
	expect(target.querySelector("path.line")?.classList.contains("negative")).toBe(true);
	await unmount(view);

	view = mount(StocksPanel, { target, props: { stocks, symbol: "AAPL", error: null } });
	await tick();
	expect(target.querySelector('[role="alert"]')?.textContent).toBe("AAPL quote unavailable");
});

it("names the selected service and lists affected components", async () => {
	view = mount(ServiceStatusPanel, {
		target,
		props: {
			catalog: [{ id: "codex", name: "OpenAI Codex", keywords: "" }],
			serviceId: "codex",
			error: null,
			report: {
				services: [
					{
						serviceId: "codex",
						name: "Codex",
						status: "partialOutage",
						operationalPercent: 66.6,
						operationalComponents: 2,
						totalComponents: 3,
						affectedComponents: [{ name: "Cloud tasks", status: "majorOutage" }],
						activeIncidents: 1,
						updatedAt: "2026-09-30T12:00:00Z",
					},
				],
				failures: [],
			},
		},
	});
	await tick();
	expect(target.textContent).toContain("OpenAI Codex");
	expect(target.textContent).toContain("Partial outage");
	expect(target.textContent).toContain("Cloud tasksMajor outage");
	expect(target.textContent).toContain("67%");
	expect(target.textContent).toContain("1 active incident");
	expect(target.querySelector('[role="progressbar"]')?.getAttribute("aria-valuenow")).toBe("66.6");
});

it("renders the matching location forecast and its failure otherwise", async () => {
	const report = {
		locations: [
			{
				query: "ningbo",
				location: "宁波",
				latitude: 29.87,
				longitude: 121.55,
				timezone: "Asia/Shanghai",
				timezoneAbbreviation: "CST",
				utcOffsetSeconds: 28_800,
				current: {
					time: "2026-09-30T12:00",
					temperature2m: 24,
					apparentTemperature: 25,
					relativeHumidity2m: 70,
					weatherCode: 0,
					windSpeed10m: 3,
					isDay: 1,
				},
				forecast: [
					{ time: "2026-09-30T13:00", temperature2m: 24.6, weatherCode: 61 },
					{ time: "2026-09-30T14:00", temperature2m: 23.2, weatherCode: 95 },
				],
			},
		],
		failures: [{ query: "atlantis", message: "No location matched atlantis" }],
	};
	view = mount(WeatherPanel, {
		target,
		props: { weather: report, location: "ningbo", error: null },
	});
	await tick();
	expect(target.textContent).toContain("宁波");
	expect(target.textContent).toContain("25°");
	expect(target.querySelector('[title="雨"]')).not.toBeNull();
	expect(target.querySelector('[title="雷雨"]')).not.toBeNull();
	await unmount(view);

	view = mount(WeatherPanel, {
		target,
		props: { weather: report, location: "atlantis", error: null },
	});
	await tick();
	expect(target.textContent).toContain("No location matched atlantis");
});

it("shows a quotation with its tags, or the read error", async () => {
	view = mount(QuotationPanel, {
		target,
		props: {
			quotation: {
				id: 1,
				content: "Stay hungry.",
				author: "Someone",
				authorSlug: "someone",
				tags: ["life", "work"],
			},
			error: null,
		},
	});
	await tick();
	expect(target.textContent).toContain("Stay hungry.");
	expect(target.textContent).toContain("life · work");
	await unmount(view);

	view = mount(QuotationPanel, { target, props: { quotation: null, error: "Quotes offline" } });
	await tick();
	expect(target.textContent).toContain("Quotes offline");
});
