import type { WidgetKind, WidgetPlacement } from "./consumer";

export interface WidgetSpan {
  columns: 3 | 4 | 6 | 8 | 12;
}

export interface WidgetDef {
  id: WidgetKind;
  label: string;
  description: string;
  category: WidgetCategory;
  span: WidgetSpan;
}

export type WidgetCategory = "system" | "quota" | "balance" | "personal" | "online";

export const widgetCategories: Array<{ id: WidgetCategory; label: string }> = [
  { id: "system", label: "System Status" },
  { id: "quota", label: "Quota" },
  { id: "balance", label: "Balance" },
  { id: "personal", label: "Personal" },
  { id: "online", label: "Online Services" },
];

export function widgetCategoryLabel(category: WidgetCategory): string {
  for (const item of widgetCategories) {
    if (item.id === category) return item.label;
  }
  return category;
}

export interface WidgetOption extends Omit<WidgetDef, "id" | "span"> {
  id: string;
  kind: WidgetKind;
  widget: WidgetPlacement["widget"];
}

export const widgets: Record<WidgetKind, WidgetDef> = {
  cpu: {
    id: "cpu",
    label: "UGREEN CPU",
    description: "Monitor UGREEN NAS processor load and temperature trends",
    category: "system",
    span: { columns: 3 },
  },
  memory: {
    id: "memory",
    label: "UGREEN Memory",
    description: "Monitor UGREEN NAS memory usage and recent changes",
    category: "system",
    span: { columns: 3 },
  },
  storage: {
    id: "storage",
    label: "UGREEN Storage",
    description: "Monitor used and available storage on UGREEN NAS",
    category: "system",
    span: { columns: 3 },
  },
  network: {
    id: "network",
    label: "UGREEN Network",
    description: "Monitor UGREEN NAS upload and download throughput",
    category: "system",
    span: { columns: 3 },
  },
  localCpu: {
    id: "localCpu",
    label: "Device CPU",
    description: "Monitor processor load on the current device",
    category: "system",
    span: { columns: 3 },
  },
  localMemory: {
    id: "localMemory",
    label: "Device Memory",
    description: "Monitor memory usage on the current device",
    category: "system",
    span: { columns: 3 },
  },
  localStorage: {
    id: "localStorage",
    label: "Device Storage",
    description: "Monitor storage usage on the current device",
    category: "system",
    span: { columns: 3 },
  },
  localNetwork: {
    id: "localNetwork",
    label: "Device Network",
    description: "Monitor network throughput on the current device",
    category: "system",
    span: { columns: 3 },
  },
  weather: {
    id: "weather",
    label: "Weather & Clock",
    description: "View local time and forecast for a location",
    category: "online",
    span: { columns: 4 },
  },
  stock: {
    id: "stock",
    label: "U.S. Stocks",
    description: "Track price, daily change, and monthly trend by symbol",
    category: "online",
    span: { columns: 3 },
  },
  exchange: {
    id: "exchange",
    label: "Exchange Rates",
    description: "View daily USD, GBP, and EUR reference rates against CNY",
    category: "online",
    span: { columns: 4 },
  },
  serviceStatus: {
    id: "serviceStatus",
    label: "Service Status",
    description: "Monitor the health of supported online services",
    category: "online",
    span: { columns: 4 },
  },
  github: {
    id: "github",
    label: "GitHub",
    description: "Summarize contributions and recent development activity",
    category: "online",
    span: { columns: 12 },
  },
  calendar: {
    id: "calendar",
    label: "Calendar",
    description: "Browse months and select the active Todo date",
    category: "personal",
    span: { columns: 4 },
  },
  todoList: {
    id: "todoList",
    label: "Todo",
    description: "Manage tasks for the selected date",
    category: "personal",
    span: { columns: 4 },
  },
  codex: {
    id: "codex",
    label: "Codex",
    description: "View Codex subscription quotas and reset times",
    category: "quota",
    span: { columns: 4 },
  },
  openCode: {
    id: "openCode",
    label: "OpenCode Go",
    description: "View OpenCode Go quotas and reset times",
    category: "quota",
    span: { columns: 4 },
  },
  claude: {
    id: "claude",
    label: "Claude",
    description: "View Claude Code quotas and reset times",
    category: "quota",
    span: { columns: 4 },
  },
  grok: {
    id: "grok",
    label: "Grok",
    description: "View Grok quotas and reset times",
    category: "quota",
    span: { columns: 4 },
  },
  copilot: {
    id: "copilot",
    label: "Copilot",
    description: "View GitHub Copilot quotas",
    category: "quota",
    span: { columns: 4 },
  },
  deepSeek: {
    id: "deepSeek",
    label: "DeepSeek",
    description: "View the DeepSeek account balance",
    category: "balance",
    span: { columns: 4 },
  },
  cherryIn: {
    id: "cherryIn",
    label: "Cherry",
    description: "View the Cherry account balance",
    category: "balance",
    span: { columns: 4 },
  },
  quotation: {
    id: "quotation",
    label: "Random Quotation",
    description: "Read a random quotation from the online provider",
    category: "online",
    span: { columns: 4 },
  },
};

const singletonKinds: Array<Exclude<WidgetKind, "weather" | "stock" | "serviceStatus">> = [
  "cpu",
  "memory",
  "storage",
  "network",
  "localCpu",
  "localMemory",
  "localStorage",
  "localNetwork",
  "exchange",
  "github",
  "calendar",
  "todoList",
  "codex",
  "openCode",
  "claude",
  "grok",
  "copilot",
  "deepSeek",
  "cherryIn",
  "quotation",
];

export const widgetOptions: WidgetOption[] = [
  ...singletonKinds.map((kind) => ({
    id: kind,
    kind,
    widget: { kind },
    label: widgets[kind].label,
    description: widgets[kind].description,
    category: widgets[kind].category,
  })),
  {
    id: "weather",
    kind: "weather",
    widget: { kind: "weather", location: "" },
    label: widgets.weather.label,
    description: widgets.weather.description,
    category: widgets.weather.category,
  },
  {
    id: "stock",
    kind: "stock",
    widget: { kind: "stock", symbol: "" },
    label: widgets.stock.label,
    description: widgets.stock.description,
    category: widgets.stock.category,
  },
  {
    id: "serviceStatus",
    kind: "serviceStatus",
    widget: { kind: "serviceStatus", serviceId: "" },
    label: widgets.serviceStatus.label,
    description: widgets.serviceStatus.description,
    category: widgets.serviceStatus.category,
  },
];

export function widgetKey(widget: WidgetPlacement["widget"]): string {
  if (widget.kind === "weather") return `weather-${widget.location.trim().toLocaleLowerCase()}`;
  if (widget.kind === "stock") return `stock-${widget.symbol.toLocaleLowerCase()}`;
  if (widget.kind === "serviceStatus") return `service-status-${widget.serviceId}`;
  return widget.kind;
}

export function widgetLabel(placement: WidgetPlacement): string {
  if (placement.widget.kind === "weather") {
    if (placement.widget.location === "shanghai") return "Shanghai Weather";
    if (placement.widget.location === "ningbo") return "Ningbo Weather";
    if (placement.widget.location === "nottingham") return "Nottingham Weather";
    return `${placement.widget.location} Weather`;
  }
  if (placement.widget.kind === "stock") return `${placement.widget.symbol} Quote`;
  if (placement.widget.kind === "serviceStatus") return `${placement.widget.serviceId} Status`;
  return widgets[placement.widget.kind].label;
}
