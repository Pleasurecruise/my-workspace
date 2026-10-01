import type { CommandResponse } from "./command";
import type { Game } from "./games";

export interface TaskManagerSnapshot {
	cpu: CpuSample | null;
	cpuHistory: CpuSample[];
	memory: MemorySample | null;
	memoryHistory: MemorySample[];
	storage: StorageSample | null;
	network: NetworkSample | null;
	networkHistory: NetworkSample[];
}

export interface CpuSample {
	usedPercent: number;
	temperature: number;
	sampledAt: number;
}

export interface MemorySample {
	usedPercent: number;
	sampledAt: number;
}

export interface StorageSample {
	usedPercent: number;
	sampledAt: number;
}

export interface NetworkSample {
	receiveRate: number;
	sendRate: number;
	sampledAt: number;
}

export interface DeviceTelemetrySnapshot {
	cpu: PercentSample;
	cpuHistory: PercentSample[];
	memory: LocalMemorySample;
	memoryHistory: PercentSample[];
	storage: LocalStorageSample | null;
	network: NetworkSample;
	networkHistory: NetworkSample[];
}

export interface PercentSample {
	usedPercent: number;
	sampledAt: number;
}

export interface LocalMemorySample extends PercentSample {
	usedBytes: number;
	totalBytes: number;
}

export interface LocalStorageSample extends PercentSample {
	usedBytes: number;
	totalBytes: number;
}

export interface CodexUsage {
	planType: string | null;
	primary: RateLimitWindow | null;
	secondary: RateLimitWindow | null;
}

export interface RateLimitWindow {
	usedPercent: number;
	windowDurationMins: number | null;
	resetsAt: number | null;
}

export interface OpenCodeUsage {
	usage: {
		rolling: OpenCodeUsageWindow;
		weekly: OpenCodeUsageWindow;
		monthly: OpenCodeUsageWindow;
	};
}

export interface OpenCodeUsageWindow {
	status: "ok" | "rate-limited";
	percent: number;
	resetsAt: string;
}

export interface ClaudeUsage {
	planType: string;
	fiveHour: ClaudeUsageWindow | null;
	sevenDay: ClaudeUsageWindow | null;
}

export interface ClaudeUsageWindow {
	usedPercent: number;
	windowDurationMins: number;
	resetsAt: string | null;
}

export interface GrokUsage {
	planType: string | null;
	window: { usedPercent: number; windowDurationMins: number | null; resetsAt: string | null };
}

export interface CopilotQuota {
	entitlement: number | null;
	remaining: number | null;
	percentRemaining: number | null;
	unlimited: boolean | null;
	overageCount: number | null;
	overagePermitted: boolean | null;
	quotaResetAt: number | null;
	timestampUtc: string | null;
}

export interface CopilotUsage {
	login: string | null;
	copilotPlan: string | null;
	accessTypeSku: string | null;
	quotaResetDateUtc: string | null;
	quotaSnapshots: {
		chat: CopilotQuota | null;
		completions: CopilotQuota | null;
		premiumInteractions: CopilotQuota | null;
	};
}

export interface DeepSeekBalance {
	isAvailable: boolean;
	balanceInfos: Array<{
		currency: "CNY" | "USD";
		totalBalance: string;
		grantedBalance: string;
		toppedUpBalance: string;
	}>;
}

export interface CherryInBalance {
	balance: number;
}

export interface DimAgentUsage {
	planName: string | null;
	credits: {
		totalUnits: number;
		usedUnits: number;
		remainingUnits: number;
		expiresAt: string | null;
	};
	featureMeters: Array<{
		featureKey: string;
		totalRemaining: number;
		unit: string | null;
		unlimited: boolean;
		totalAllowance: number;
		totalUsed: number;
		periodEnd: string | null;
	}>;
}

export interface TokenFluxUsage {
	remaining: number;
	unit: string;
	planName: string;
	isValid: boolean;
	mode: string;
	billing: {
		available: boolean;
		mode: string;
		planId: number;
		planName: string;
		preferredSubscriptionId: number | null;
		remaining: number;
		source: string | null;
		subscriptionId: number;
		unit: string;
	};
	subscription: {
		dailyLimitUsd: number | null;
		dailyUsageUsd: number;
		expiresAt: string | null;
		id: number;
		monthlyLimitUsd: number;
		monthlyUsageUsd: number;
		planId: number;
		status: string;
		weeklyLimitUsd: number | null;
		weeklyUsageUsd: number;
		weeklyWindowStart: string | null;
	};
	usage: {
		averageDurationMs: number;
		rpm: number;
		tpm: number;
		today: TokenFluxUsageWindow;
		total: TokenFluxUsageWindow;
	};
}

export interface TokenFluxUsageWindow {
	actualCost: number;
	cacheCreationTokens: number;
	cacheReadTokens: number;
	cost: number;
	inputTokens: number;
	outputTokens: number;
	requests: number;
	totalTokens: number;
}

export interface Weather {
	query: string;
	location: string;
	latitude: number;
	longitude: number;
	timezone: string;
	timezoneAbbreviation: string;
	utcOffsetSeconds: number;
	current: {
		time: string;
		temperature2m: number;
		apparentTemperature: number;
		relativeHumidity2m: number;
		weatherCode: number;
		windSpeed10m: number;
		isDay: number;
	};
	forecast: Array<{
		time: string;
		temperature2m: number;
		weatherCode: number;
	}>;
}

export interface WeatherReport {
	locations: Weather[];
	failures: Array<{ query: string; message: string }>;
}

export type WeatherLocation = string;

export interface StockReport {
	stocks: StockSeries[];
	failures: Array<{ symbol: string; message: string }>;
}

export interface StockSeries {
	symbol: string;
	name: string;
	currency: string;
	exchange: string;
	price: number;
	change: number;
	changePercent: number;
	points: [
		{ timestamp: number; close: number },
		{ timestamp: number; close: number },
		...Array<{ timestamp: number; close: number }>,
	];
}

export interface ExchangeReport {
	referenceCurrency: "EUR";
	preparedAt: string;
	rates: ExchangeRate[];
}

export interface ExchangeRate {
	code: string;
	name: string;
	date: string;
	unitsPerEuro: number;
	previousUnitsPerEuro: number;
	change: number;
	changePercent: number;
}

export type ServiceStatusLevel =
	| "operational"
	| "underMaintenance"
	| "degradedPerformance"
	| "partialOutage"
	| "majorOutage"
	| "unknown";

export interface ServiceStatusCatalogEntry {
	id: string;
	name: string;
	keywords: string;
}

export interface ServiceStatusReport {
	services: Array<{
		serviceId: string;
		name: string;
		status: ServiceStatusLevel;
		operationalPercent: number;
		operationalComponents: number;
		totalComponents: number;
		affectedComponents: Array<{ name: string; status: ServiceStatusLevel }>;
		activeIncidents: number;
		updatedAt: string;
	}>;
	failures: Array<{ serviceId: string; message: string }>;
}

export interface GithubSnapshot {
	login: string;
	profileUrl: string;
	totalContributions: number;
	weeks: Array<{
		days: Array<{ date: string; count: number; level: number }>;
	}>;
	recentActivity: GithubActivity[];
	notifications:
		| { status: "ready"; items: GithubNotification[]; hasMore: boolean }
		| { status: "failed"; message: string };
}

export interface GithubNotification {
	id: string;
	title: string;
	repository: string;
	reason: string;
	updatedAt: string;
	url: string | null;
}

export interface Quotation {
	id: number;
	content: string;
	author: string;
	authorSlug: string;
	tags: string[];
}

export interface GithubActivity {
	kind: "commit" | "pullRequest" | "review" | "approve";
	title: string;
	repository: string;
	occurredAt: string;
	url: string;
}

export interface QueryState<T> {
	data: T | null;
	error: string | null;
	loading: boolean;
}

export interface DashboardState {
	taskManager: QueryState<TaskManagerSnapshot>;
	deviceTelemetry: QueryState<DeviceTelemetrySnapshot>;
	codex: QueryState<CodexUsage>;
	openCode: QueryState<OpenCodeUsage>;
	claude: QueryState<ClaudeUsage>;
	grok: QueryState<GrokUsage>;
	copilot: QueryState<CopilotUsage>;
	deepSeek: QueryState<DeepSeekBalance>;
	cherryIn: QueryState<CherryInBalance>;
	tokenFlux: QueryState<TokenFluxUsage>;
	dimAgent: QueryState<DimAgentUsage>;
	weather: QueryState<WeatherReport>;
	stocks: QueryState<StockReport>;
	exchange: QueryState<ExchangeReport>;
	serviceStatus: QueryState<ServiceStatusReport>;
	github: QueryState<GithubSnapshot>;
	quotation: QueryState<Quotation>;
}

export type WidgetKind =
	| "invalid"
	| "cpu"
	| "memory"
	| "storage"
	| "network"
	| "localCpu"
	| "localMemory"
	| "localStorage"
	| "localNetwork"
	| "weather"
	| "stock"
	| "exchange"
	| "serviceStatus"
	| "github"
	| "planner"
	| "spending"
	| "codex"
	| "openCode"
	| "claude"
	| "codexClaude"
	| "grok"
	| "copilot"
	| "deepSeek"
	| "cherryIn"
	| "tokenFlux"
	| "dimAgent"
	| "quotation"
	| "game"
	| "steam";

export interface WidgetPlacement {
	id: string;
	widget:
		| {
				kind: Exclude<
					WidgetKind,
					"weather" | "stock" | "serviceStatus" | "game" | "planner" | "invalid"
				>;
		  }
		| { kind: "game"; game: Game }
		| { kind: "weather"; location: WeatherLocation }
		| { kind: "stock"; symbol: string }
		| { kind: "serviceStatus"; serviceId: string }
		| { kind: "planner"; habits: Habit[] }
		| { kind: "invalid"; configuration: string; error: string };
}

export interface WidgetLayout {
	widgets: WidgetPlacement[];
	islandWidgetId: string | null;
}

export type DashboardEvent =
	| { source: "games"; result: CommandResponse<null> }
	| { source: "taskManager"; result: CommandResponse<TaskManagerSnapshot | null> }
	| { source: "deviceTelemetry"; result: CommandResponse<DeviceTelemetrySnapshot | null> }
	| { source: "codex"; result: CommandResponse<CodexUsage | null> }
	| { source: "openCode"; result: CommandResponse<OpenCodeUsage | null> }
	| { source: "claude"; result: CommandResponse<ClaudeUsage | null> }
	| { source: "grok"; result: CommandResponse<GrokUsage | null> }
	| { source: "copilot"; result: CommandResponse<CopilotUsage | null> }
	| { source: "deepSeek"; result: CommandResponse<DeepSeekBalance | null> }
	| { source: "cherryIn"; result: CommandResponse<CherryInBalance | null> }
	| { source: "tokenFlux"; result: CommandResponse<TokenFluxUsage | null> }
	| { source: "dimAgent"; result: CommandResponse<DimAgentUsage | null> }
	| { source: "weather"; result: CommandResponse<WeatherReport> }
	| { source: "stocks"; result: CommandResponse<StockReport> }
	| { source: "exchange"; result: CommandResponse<ExchangeReport | null> }
	| { source: "serviceStatus"; result: CommandResponse<ServiceStatusReport> }
	| { source: "github"; result: CommandResponse<GithubSnapshot | null> }
	| { source: "quotation"; result: CommandResponse<Quotation | null> };

export interface Habit {
	id: string;
	name: string;
}
