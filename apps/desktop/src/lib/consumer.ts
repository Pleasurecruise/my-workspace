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
  thumbHash: string;
  width: number;
  height: number;
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
  source: string;
  html: string;
  toc: TocEntry[];
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
  shanghai: Weather;
  ningbo: Weather;
  nottingham: Weather;
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
  codex: QueryState<CodexUsage>;
  openCode: QueryState<OpenCodeUsage>;
  deepSeek: QueryState<DeepSeekBalance>;
  cherryIn: QueryState<CherryInBalance>;
  weather: QueryState<WeatherReport>;
  github: QueryState<GithubSnapshot>;
}

export interface DashboardQueryResults {
  read_task_manager: TaskManagerSnapshot;
  read_codex_usage: CodexUsage;
  read_opencode_usage: OpenCodeUsage;
  read_deepseek_balance: DeepSeekBalance;
  read_cherryin_balance: CherryInBalance;
  read_weather: WeatherReport;
  read_github: GithubSnapshot;
}

export interface TodoList {
  date: string;
  items: Array<{
    id: string;
    text: string;
    completed: boolean;
  }>;
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
}
