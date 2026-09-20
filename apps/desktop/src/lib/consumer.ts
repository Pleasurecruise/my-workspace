export interface IslandGeometry {
	topInset: number;
	notchWidth: number;
}

export type Channel = "memos" | "moment" | "knowledge";

export interface Memo {
	id: string;
	r2Key: string;
	content: string;
	tags: string[];
	createdAt: string;
	updatedAt: string;
	visibility: "public" | "private";
	pinned: boolean;
	favorite: boolean;
	archived: boolean;
}

export interface MemoView extends Memo {
	html: string;
	metadataComplete: boolean;
}

export interface PublishedPost {
	provider: "telegram" | "x";
	externalId: string;
	url: string | null;
}

export interface MemoTagCount {
	name: string;
	count: number;
}

export type MemoUpdate =
	| { content: string; visibility: "public" | "private" }
	| { tags: string[] }
	| { pinned: boolean }
	| { favorite: boolean }
	| { archived: boolean };

export interface PhotoItem {
	id: string;
	url: string;
	thumbnailUrl: string;
	r2Key: string;
	thumbnailR2Key: string;
	thumbHash: string | null;
	title: string;
	width: number;
	height: number;
	aspectRatio: number | null;
	tags: string[];
	date: string | null;
	description: string | null;
	size: number | null;
	format: string | null;
	geo: { lat: number; lng: number } | null;
}

export interface PhotoMetadata {
	capturedAt: string | null;
	geo: { lat: number; lng: number } | null;
}

export interface PhotoUpload {
	title: string;
	description: string | null;
	tags: string[];
	date: string | null;
	geo: { lat: number; lng: number } | null;
}

export interface PhotoUpdate {
	title: string;
	description: string;
	tags: string[];
}

export type ChannelView =
	| {
			channel: "memos";
			memos: MemoView[];
			nextCursor: string | null;
	  }
	| {
			channel: "moment";
			photos: PhotoItem[];
			total: number;
	  }
	| {
			channel: "knowledge";
			knowledge: KnowledgeEntry[];
			newspaper: NewspaperIssues;
			nextCursor: string | null;
	  };

export interface TocEntry {
	id: string;
	text: string;
	depth: number;
}

export interface KnowledgeEntry {
	id: string;
	title: string;
	summary: string;
	tags: string[];
	visibility: "private" | "public";
	contentHash: string;
	createdAt: string;
	updatedAt: string;
	newspaperEdition: "developer" | "personal" | null;
}

export interface ReadingStats {
	wordCount: number;
	readingMinutes: number;
}

export interface KnowledgeDocument extends KnowledgeEntry {
	source: string;
	html: string;
	toc: TocEntry[];
	stats: ReadingStats;
}

export interface NewspaperIssues {
	developer: string | null;
	personal: string | null;
}

export interface KnowledgeDraft {
	title: string;
	summary: string;
	body: string;
	tags: string[];
}

export interface KnowledgeUpdate extends KnowledgeDraft {
	expectedHash: string;
	expectedUpdatedAt: string;
	visibility?: KnowledgeDocument["visibility"];
}

export type CommandResponse<T> =
	| { status: "ready"; data: T }
	| { status: "failed"; message: string };

export interface UpdateInfo {
	currentVersion: string;
	version: string;
	notes: string | null;
}

export type UpdateProgress =
	| { status: "downloading"; downloaded: number; total: number | null }
	| { status: "downloaded" };

export interface InitialViews {
	memos: CommandResponse<ChannelView>;
	moment: CommandResponse<ChannelView>;
	knowledge: CommandResponse<ChannelView>;
}

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

export type Game = "genshin" | "starRail" | "zzz" | "arknights" | "endfield";
export type GameProvider = "mihoyo" | "skland" | "steam";
export interface GameConnections {
	providers: GameProvider[];
	mihoyo: string[];
	bindings: Array<{ game: Game; accountId: string | null }>;
}
export interface GameLoginQr {
	id: string;
	image: string;
	expiresAt: number;
}
export type GameLoginProgress = "waiting" | "scanned" | "expired" | "complete";
export interface GameNotes {
	account: { game: Game; uid: string; region: string; name: string; roleId: string };
	sampledAt: number;
	meters: Array<{ label: string; current: number; max: number; fullAt: number | null }>;
	tasks: Array<{
		label: string;
		value: string;
		progress: { current: number; total: number } | null;
	}>;
}

export type GameNotesResponse =
	| { status: "ready"; data: GameNotes }
	| { status: "verificationRequired"; code: number; message: string }
	| { status: "refreshRequired"; message: string }
	| { status: "failed"; message: string };
export interface GachaPull {
	id: string;
	pool: string;
	poolName: string;
	itemId: string | null;
	name: string;
	rarity: number;
	time: string;
	isFree: boolean | null;
	isNew: boolean | null;
}
export interface GachaArchive {
	game: Game;
	uid: string | null;
	accounts: Array<{ uid: string; name: string }>;
	total: number;
	added: number;
	syncedAt: number | null;
	firstRecord: string | null;
	lastRecord: string | null;
	pools: Array<{
		id: string;
		name: string;
		total: number;
		highRarity: number;
		rarities: [number, number, number, number, number, number];
		freePulls: number;
		sinceHighRarity: number;
		averageInterval: number | null;
	}>;
	recent: GachaPull[];
	official: StarRailReport | null;
}
export interface StarRailReport {
	pools: Array<{
		id: string;
		name: string;
		total: number;
		sinceHighRarity: number | null;
		fiveStars: Array<{ id: string; itemId: number; name: string; pulls: number; isUp: boolean }>;
	}>;
}
export interface SteamSettings {
	apiKey: string;
	steamId: string;
}

export interface SteamGames {
	name: string;
	profileUrl: string;
	state: number | null;
	playing: string | null;
	ownedGames: number | null;
	recentCount: number | null;
	recentMinutes: number | null;
	totalMinutes: number | null;
	playedGames: number | null;
	mostPlayed: SteamGames["recent"];
	sampledAt: number;
	recent: Array<{
		appId: number;
		name: string;
		playtimeForever: number;
		playtime2weeks: number | null;
	}>;
}

export interface TodoDetails {
	calendar: string;
	startDate: string;
	startTime: string | null;
	endDate: string | null;
	endTime: string | null;
	location: string | null;
}

export interface TodoItem {
	rollover: boolean;
	id: string;
	description: string | null;
	text: string;
	completed: boolean;
	details: TodoDetails | null;
}

export interface TodoList {
	syncError: string | null;
	date: string;
	items: TodoItem[];
}

export interface UgosConfiguration {
	username: string;
	password: string;
}

export interface R2Configuration {
	accessKeyId: string;
	secretAccessKey: string;
}

export interface ApiConfiguration {
	service: "memos" | "moment" | "knowledge";
	apiKey: string;
}

export interface CodexResets {
	enabled: boolean;
}

export interface NotionCalendar {
	viewUrl: string;
}

export interface NtfyConfig {
	token: string;
	development: boolean;
}

export interface TelegramCredentials {
	apiId: number;
	apiHash: string;
	channelUsername: string;
}

export type TelegramAuthorizationStatus =
	| { status: "disconnected" }
	| { status: "ready" }
	| { status: "codeRequired" }
	| { status: "passwordRequired"; hint: string | null };

export interface NtfyNotification {
	id: string;
	topic: string;
	source: string;
	title: string | null;
	message: string;
	timestamp: number;
	tags: string[];
}

export type StoredConfiguration<T> = { status: "missing" } | { status: "ready"; data: T };

export interface ConfigurationStatus {
	ugos: StoredConfiguration<UgosConfiguration>;
	r2: StoredConfiguration<R2Configuration>;
	api: {
		memos: StoredConfiguration<string>;
		moment: StoredConfiguration<string>;
		knowledge: StoredConfiguration<string>;
	};
	ntfy: StoredConfiguration<NtfyConfig>;
	ntfyDev: boolean;
	notionCalendar: StoredConfiguration<NotionCalendar>;
	codexResets: CodexResets;
	appLock: StoredConfiguration<string>;
	appLockDev: boolean;
	spotify: StoredConfiguration<string>;
	qqMusic: StoredConfiguration<string>;
	publication: { telegram: boolean; x: boolean };
}

export type MusicProvider = "spotify" | "qqMusic";

export interface QqQr {
	image: string;
}

export type QqLoginStatus =
	| { status: "waiting" }
	| { status: "scanned" }
	| { status: "complete" }
	| { status: "expired" };

export interface MusicTrack {
	id: string;
	name: string;
	artists: string[];
	album: string;
	durationMs: number;
	addedAt: string;
	coverKey: string | null;
}

export interface MusicPlayback {
	trackId: string | null;
	playing: boolean;
	progressMs: number;
	durationMs: number;
	order: "sequential" | "repeatOne" | "shuffle";
}

export interface MusicLyrics {
	lines: Array<{ startMs: number | null; text: string }>;
	synced: boolean;
	instrumental: boolean;
}

export interface CheckIn {
	id: string;
	editable: boolean;
	date: string;
	completed: boolean;
	streak: number;
	total: number;
	days: Array<{ date: string; completed: boolean }>;
}

export interface Habit {
	id: string;
	name: string;
}

export interface ExpenseEntry {
	description: string | null;
	id: string;
	date: string;
	amountPence: number;
	category: string;
}

export interface ExpenseSnapshot {
	date: string;
	month: string;
	entries: ExpenseEntry[];
	dayTotalPence: number;
	monthTotalPence: number;
	categories: Array<{ category: string; amountPence: number }>;
	days: Array<{ date: string; amountPence: number }>;
	suggestions: string[];
}

export interface SshDevice {
	id: string;
	name: string;
	dnsName: string;
	address: string;
	os: string;
	online: boolean | null;
	username: string;
}
export type TerminalTarget = { kind: "local" } | { kind: "ssh"; device: SshDevice };
export type TerminalConnection =
	| { kind: "local" }
	| { kind: "ssh"; deviceId: string; username: string };

export interface SshSnapshot {
	devices: SshDevice[];
	error: string | null;
}
export type TerminalOutput =
	| { kind: "data"; bytes: number[] }
	| { kind: "exit"; code: number | null }
	| { kind: "error"; message: string };
