import type { WidgetKind, WidgetPlacement } from "./consumer";

export interface WidgetSpan {
  columns: 3 | 4 | 6 | 8 | 12;
}

export interface WidgetDef {
  id: WidgetKind;
  label: string;
  description: string;
  category: "system" | "personal" | "online";
  span: WidgetSpan;
}

export interface WidgetOption extends Omit<WidgetDef, "id" | "span"> {
  id: string;
  kind: WidgetKind;
  widget: WidgetPlacement["widget"];
}

export const weatherLocations: Record<string, string> = {
  shanghai: "上海",
  ningbo: "宁波",
  nottingham: "诺丁汉",
};

export const widgets: Record<WidgetKind, WidgetDef> = {
  cpu: {
    id: "cpu",
    label: "CPU",
    description: "实时查看处理器负载与温度趋势",
    category: "system",
    span: { columns: 3 },
  },
  memory: {
    id: "memory",
    label: "内存",
    description: "查看内存占用与近期变化",
    category: "system",
    span: { columns: 3 },
  },
  storage: {
    id: "storage",
    label: "存储",
    description: "快速了解磁盘已用和可用空间",
    category: "system",
    span: { columns: 3 },
  },
  network: {
    id: "network",
    label: "网络",
    description: "显示实时上传、下载速率趋势",
    category: "system",
    span: { columns: 3 },
  },
  weather: {
    id: "weather",
    label: "天气与时钟",
    description: "查看一个地点的当地时间和未来天气",
    category: "online",
    span: { columns: 4 },
  },
  stock: {
    id: "stock",
    label: "美股行情",
    description: "输入美股代码，查看价格、涨跌和月度走势",
    category: "online",
    span: { columns: 3 },
  },
  github: {
    id: "github",
    label: "GitHub",
    description: "汇总贡献记录与最近的开发动态",
    category: "online",
    span: { columns: 12 },
  },
  todo: {
    id: "todo",
    label: "Todo",
    description: "在月历旁管理每日待办事项",
    category: "personal",
    span: { columns: 4 },
  },
  usage: {
    id: "usage",
    label: "额度与余额",
    description: "集中查看 AI 服务额度与账户余额",
    category: "personal",
    span: { columns: 8 },
  },
};

const singletonKinds: Array<Exclude<WidgetKind, "weather" | "stock">> = [
  "cpu",
  "memory",
  "storage",
  "network",
  "github",
  "todo",
  "usage",
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
];

export function widgetKey(widget: WidgetPlacement["widget"]): string {
  if (widget.kind === "weather") return `weather-${widget.location.trim().toLocaleLowerCase()}`;
  if (widget.kind === "stock") return `stock-${widget.symbol.toLocaleLowerCase()}`;
  return widget.kind;
}

export function widgetLabel(placement: WidgetPlacement): string {
  if (placement.widget.kind === "weather")
    return `${weatherLocations[placement.widget.location] ?? placement.widget.location}天气`;
  if (placement.widget.kind === "stock") return `${placement.widget.symbol} 行情`;
  return widgets[placement.widget.kind].label;
}
