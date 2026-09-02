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
      connected: boolean;
      memos: MemoView[];
      tags: MemoTagCount[];
      nextCursor: string | null;
    }
  | {
      channel: "moment";
      connected: boolean;
      photos: PhotoItem[];
      tags: string[];
      total: number;
      nextCursor: string | null;
    }
  | {
      channel: "knowledge";
      connected: boolean;
      knowledge: KnowledgeDocument[];
      newspaper: NewspaperIssues;
      nextCursor: string | null;
    };

export interface TocEntry {
  id: string;
  text: string;
  depth: number;
}

export interface KnowledgeDocument {
  id: string;
  slug: string;
  title: string;
  summary: string;
  tags: string[];
  visibility: "private" | "public";
  contentHash: string;
  createdAt: string;
  updatedAt: string;
  newspaperEdition: "developer" | "personal" | null;
  source: string;
  html: string;
  toc: TocEntry[];
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
  spark: CodexLimit | null;
}

export interface CodexLimit {
  limitId: string | null;
  limitName: string | null;
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
  weather: QueryState<WeatherReport>;
  stocks: QueryState<StockReport>;
  exchange: QueryState<ExchangeReport>;
  serviceStatus: QueryState<ServiceStatusReport>;
  github: QueryState<GithubSnapshot>;
  quotation: QueryState<Quotation>;
}

export type WidgetKind =
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
  | "calendar"
  | "todoList"
  | "codex"
  | "openCode"
  | "claude"
  | "grok"
  | "copilot"
  | "deepSeek"
  | "cherryIn"
  | "quotation";

export interface WidgetPlacement {
  id: string;
  widget:
    | { kind: Exclude<WidgetKind, "weather" | "stock" | "serviceStatus"> }
    | { kind: "weather"; location: WeatherLocation }
    | { kind: "stock"; symbol: string }
    | { kind: "serviceStatus"; serviceId: string };
}

export interface WidgetLayout {
  widgets: WidgetPlacement[];
}

export type DashboardEvent =
  | { source: "taskManager"; result: CommandResponse<TaskManagerSnapshot> }
  | { source: "deviceTelemetry"; result: CommandResponse<DeviceTelemetrySnapshot | null> }
  | { source: "codex"; result: CommandResponse<CodexUsage> }
  | { source: "openCode"; result: CommandResponse<OpenCodeUsage> }
  | { source: "claude"; result: CommandResponse<ClaudeUsage | null> }
  | { source: "grok"; result: CommandResponse<GrokUsage | null> }
  | { source: "copilot"; result: CommandResponse<CopilotUsage | null> }
  | { source: "deepSeek"; result: CommandResponse<DeepSeekBalance> }
  | { source: "cherryIn"; result: CommandResponse<CherryInBalance> }
  | { source: "weather"; result: CommandResponse<WeatherReport> }
  | { source: "stocks"; result: CommandResponse<StockReport> }
  | { source: "exchange"; result: CommandResponse<ExchangeReport | null> }
  | { source: "serviceStatus"; result: CommandResponse<ServiceStatusReport> }
  | { source: "github"; result: CommandResponse<GithubSnapshot> }
  | { source: "quotation"; result: CommandResponse<Quotation | null> };

export interface TodoDetails {
  calendar: string;
  startDate: string;
  startTime: string | null;
  endDate: string | null;
  endTime: string | null;
  location: string | null;
  description: string | null;
}

export interface TodoItem {
  id: string;
  text: string;
  completed: boolean;
  details: TodoDetails | null;
}

export interface TodoList {
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
