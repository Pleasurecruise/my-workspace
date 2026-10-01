use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{self, watch};
use tokio::task::JoinSet;
use tokio::time::{Instant, interval_at};

use crate::{CommandResponse, telemetry};
use dashboard::{Layout, Provider, Widget};
use market_data::{exchange, stocks};

const EVENT: &str = "dashboard-source-updated";
const SOURCE_COUNT: usize = 18;

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
enum Source {
    TaskManager,
    DeviceTelemetry,
    Codex,
    OpenCode,
    Claude,
    Grok,
    Copilot,
    DeepSeek,
    CherryIn,
    TokenFlux,
    DimAgent,
    Weather,
    Stocks,
    Exchange,
    ServiceStatus,
    Github,
    Quotation,
    Games,
}

impl Source {
    const ALL: [Self; SOURCE_COUNT] = [
        Self::TaskManager,
        Self::DeviceTelemetry,
        Self::Codex,
        Self::OpenCode,
        Self::Claude,
        Self::Grok,
        Self::Copilot,
        Self::DeepSeek,
        Self::CherryIn,
        Self::TokenFlux,
        Self::DimAgent,
        Self::Weather,
        Self::Stocks,
        Self::Exchange,
        Self::ServiceStatus,
        Self::Github,
        Self::Quotation,
        Self::Games,
    ];
}

#[derive(serde::Serialize)]
#[serde(tag = "source", content = "result", rename_all = "camelCase")]
enum DashboardEvent {
    TaskManager(CommandResponse<Option<ugos::TaskManagerSnapshot>>),
    DeviceTelemetry(CommandResponse<Option<telemetry::Snapshot>>),
    Codex(CommandResponse<Option<useage::codex::CodexUsage>>),
    OpenCode(CommandResponse<Option<useage::opencode::OpenCodeUsage>>),
    Claude(CommandResponse<Option<useage::claude::ClaudeUsage>>),
    Grok(CommandResponse<Option<useage::grok::GrokUsage>>),
    Copilot(CommandResponse<Option<useage::copilot::CopilotUsage>>),
    DeepSeek(CommandResponse<Option<useage::deepseek::DeepSeekBalance>>),
    CherryIn(CommandResponse<Option<useage::cherryin::CherryInBalance>>),
    TokenFlux(CommandResponse<Option<useage::tokenflux::TokenFluxUsage>>),
    DimAgent(CommandResponse<Option<useage::dimagent::DimAgentUsage>>),
    Weather(Box<CommandResponse<weather::WeatherReport>>),
    Stocks(Box<CommandResponse<stocks::StockReport>>),
    Exchange(Box<CommandResponse<Option<exchange::ExchangeReport>>>),
    ServiceStatus(Box<CommandResponse<service_status::ServiceStatusReport>>),
    Github(CommandResponse<Option<github::GithubSnapshot>>),
    Quotation(CommandResponse<Option<quotes::Quotation>>),
    Games(CommandResponse<()>),
}

impl DashboardEvent {
    async fn read(
        source: Source,
        app: &AppHandle,
        layout: &Result<Layout, String>,
        refresh_games: bool,
    ) -> Self {
        let has = |check: fn(&Layout) -> bool| layout.as_ref().map(check).map_err(Clone::clone);
        let provider = |provider: Provider| {
            layout
                .as_ref()
                .map(|layout| layout.has_provider(provider))
                .map_err(Clone::clone)
        };
        let list =
            |items: fn(&Layout) -> Vec<String>| layout.as_ref().map(items).map_err(Clone::clone);
        match source {
            Source::Games => Self::Games(match layout {
                Ok(layout) => crate::games::refresh(app, layout, refresh_games)
                    .await
                    .into(),
                Err(message) => CommandResponse::Failed {
                    message: message.clone(),
                },
            }),
            Source::TaskManager => Self::TaskManager(
                optional(source, has(Layout::has_ugos), ugos::task_manager()).await,
            ),
            Source::DeviceTelemetry => Self::DeviceTelemetry(
                optional(source, has(Layout::has_device_telemetry), telemetry::read()).await,
            ),
            Source::Codex => Self::Codex(
                optional(source, provider(Provider::Codex), useage::codex::read()).await,
            ),
            Source::OpenCode => Self::OpenCode(
                optional(
                    source,
                    provider(Provider::OpenCode),
                    useage::opencode::read(),
                )
                .await,
            ),
            Source::Claude => Self::Claude(
                optional(source, provider(Provider::Claude), useage::claude::read()).await,
            ),
            Source::Grok => {
                Self::Grok(optional(source, provider(Provider::Grok), useage::grok::read()).await)
            }
            Source::Copilot => Self::Copilot(
                optional(source, provider(Provider::Copilot), useage::copilot::read()).await,
            ),
            Source::DeepSeek => Self::DeepSeek(
                optional(
                    source,
                    provider(Provider::DeepSeek),
                    useage::deepseek::read(),
                )
                .await,
            ),
            Source::CherryIn => Self::CherryIn(
                optional(
                    source,
                    provider(Provider::CherryIn),
                    useage::cherryin::read(),
                )
                .await,
            ),
            Source::TokenFlux => Self::TokenFlux(
                optional(
                    source,
                    provider(Provider::TokenFlux),
                    useage::tokenflux::read(),
                )
                .await,
            ),
            Source::DimAgent => Self::DimAgent(
                optional(
                    source,
                    provider(Provider::DimAgent),
                    useage::dimagent::read(),
                )
                .await,
            ),
            Source::Github => {
                Self::Github(optional(source, provider(Provider::Github), github::read()).await)
            }
            Source::Exchange => Self::Exchange(Box::new(
                optional(source, has(Layout::has_exchange), exchange::read()).await,
            )),
            Source::Quotation => {
                Self::Quotation(optional(source, has(Layout::has_quotation), quotes::read()).await)
            }
            Source::Weather => Self::Weather(Box::new(
                listed(source, list(Layout::weather_locations), weather::read).await,
            )),
            Source::Stocks => Self::Stocks(Box::new(
                listed(source, list(Layout::stock_symbols), stocks::read).await,
            )),
            Source::ServiceStatus => Self::ServiceStatus(Box::new(
                listed(source, list(Layout::service_ids), service_status::read).await,
            )),
        }
    }

    fn emit(self, app: &AppHandle) {
        let payload = match serde_json::to_value(self) {
            Ok(payload) => payload,
            Err(error) => {
                tracing::error!(%error, "failed to serialize a Dashboard source event");
                return;
            }
        };
        if let Err(error) = app.emit(EVENT, payload) {
            tracing::warn!(%error, "failed to emit a Dashboard source event");
        }
    }
}

pub(crate) fn layout() -> Result<Layout, String> {
    let path = database::path()
        .map_err(|error| format!("Could not resolve Dashboard database: {error}"))?;
    dashboard::read(&path)
}

fn logged<T, E: std::fmt::Display>(source: Source, result: Result<T, E>) -> Result<T, E> {
    if let Err(error) = &result {
        tracing::warn!(?source, %error, "Dashboard source unavailable");
    }
    result
}

// A removed widget must not read credentials, start a CLI, or renew an OAuth session.
async fn optional<T, E: std::fmt::Display>(
    source: Source,
    enabled: Result<bool, String>,
    read: impl Future<Output = Result<T, E>>,
) -> CommandResponse<Option<T>> {
    match enabled {
        Ok(false) => CommandResponse::Ready { data: None },
        Ok(true) => logged(source, read.await).map(Some).into(),
        Err(message) => CommandResponse::Failed { message },
    }
}

async fn listed<T, F: Future<Output = Result<T, String>>>(
    source: Source,
    items: Result<Vec<String>, String>,
    read: impl FnOnce(Vec<String>) -> F,
) -> CommandResponse<T> {
    match items {
        Ok(items) => logged(source, read(items).await).into(),
        Err(message) => CommandResponse::Failed { message },
    }
}

struct RuntimeState {
    active: watch::Sender<bool>,
    sources: [Arc<sync::Mutex<()>>; SOURCE_COUNT],
    polling: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub(crate) struct Runtime(Arc<RuntimeState>);

impl Default for Runtime {
    fn default() -> Self {
        Self(Arc::new(RuntimeState {
            active: watch::channel(false).0,
            sources: std::array::from_fn(|_| Arc::new(sync::Mutex::new(()))),
            polling: Mutex::new(None),
        }))
    }
}

impl Runtime {
    fn poll(&self, app: &AppHandle, layout: &Arc<Result<Layout, String>>, source: Source) {
        let active = self.0.active.subscribe();
        let Ok(source_guard) = Arc::clone(&self.0.sources[source as usize]).try_lock_owned() else {
            return;
        };
        let app = app.clone();
        let layout = Arc::clone(layout);
        tauri::async_runtime::spawn(async move {
            if let Some(event) =
                read_while_active(active, DashboardEvent::read(source, &app, &layout, false)).await
            {
                event.emit(&app);
            }
            drop(source_guard);
        });
    }
}

// A route transition invalidates queued and in-flight reads, including a quick leave/re-entry.
async fn read_while_active<T>(
    mut active: watch::Receiver<bool>,
    read: impl std::future::Future<Output = T>,
) -> Option<T> {
    if !*active.borrow() || active.has_changed().unwrap_or(true) {
        return None;
    }
    tokio::select! {
        biased;
        _ = active.changed() => None,
        result = read => Some(result),
    }
}

fn island_sources(widget: &Widget) -> Vec<Source> {
    match widget {
        Widget::Cpu | Widget::Memory | Widget::Storage | Widget::Network => {
            vec![Source::TaskManager]
        }
        Widget::LocalCpu | Widget::LocalMemory | Widget::LocalStorage | Widget::LocalNetwork => {
            vec![Source::DeviceTelemetry]
        }
        Widget::Weather { .. } => vec![Source::Weather],
        Widget::Stock { .. } => vec![Source::Stocks],
        Widget::Exchange => vec![Source::Exchange],
        Widget::ServiceStatus { .. } => vec![Source::ServiceStatus],
        Widget::Github => vec![Source::Github],
        Widget::Codex => vec![Source::Codex],
        Widget::OpenCode => vec![Source::OpenCode],
        Widget::Claude => vec![Source::Claude],
        Widget::CodexClaude => vec![Source::Codex, Source::Claude],
        Widget::Grok => vec![Source::Grok],
        Widget::Copilot => vec![Source::Copilot],
        Widget::DeepSeek => vec![Source::DeepSeek],
        Widget::CherryIn => vec![Source::CherryIn],
        Widget::TokenFlux => vec![Source::TokenFlux],
        Widget::DimAgent => vec![Source::DimAgent],
        Widget::Quotation => vec![Source::Quotation],
        // Todo is read through its own session; game panels own their initial read.
        Widget::Invalid { .. }
        | Widget::Planner { .. }
        | Widget::Spending
        | Widget::Game { .. }
        | Widget::Steam => Vec::new(),
    }
}

#[tauri::command]
pub(crate) async fn refresh_island(app: AppHandle) -> CommandResponse<()> {
    let layout = layout();
    let sources = match &layout {
        Ok(layout) => layout.island().map(island_sources).unwrap_or_default(),
        Err(message) => {
            return CommandResponse::Failed {
                message: message.clone(),
            };
        }
    };
    let runtime = app.state::<Runtime>();
    futures_util::future::join_all(sources.into_iter().map(async |source| {
        let _guard = runtime.0.sources[source as usize].lock().await;
        DashboardEvent::read(source, &app, &layout, false)
            .await
            .emit(&app);
    }))
    .await;
    CommandResponse::Ready { data: () }
}

#[tauri::command]
pub(crate) async fn refresh_dashboard(
    app: AppHandle,
    refresh_games: Option<bool>,
) -> CommandResponse<()> {
    let runtime = app.state::<Runtime>().inner().clone();
    let mut active = runtime.0.active.subscribe();
    if !*active.borrow_and_update() {
        return CommandResponse::Ready { data: () };
    }
    let layout = Arc::new(layout());
    let mut requests = JoinSet::new();
    for source in Source::ALL {
        let source_lock = Arc::clone(&runtime.0.sources[source as usize]);
        let request_app = app.clone();
        let source_active = active.clone();
        let layout = Arc::clone(&layout);
        requests.spawn(async move {
            read_while_active(source_active, async {
                let source_guard = source_lock.lock_owned().await;
                let event = DashboardEvent::read(
                    source,
                    &request_app,
                    &layout,
                    refresh_games.unwrap_or(false),
                )
                .await;
                event.emit(&request_app);
                drop(source_guard);
            })
            .await;
        });
    }

    loop {
        tokio::select! {
            biased;
            _ = active.changed() => break,
            request = requests.join_next() => match request {
                Some(Ok(())) => {},
                Some(Err(error)) => tracing::error!(%error, "Dashboard source task failed"),
                None => break,
            },
        }
    }
    CommandResponse::Ready { data: () }
}

#[tauri::command]
pub(crate) fn set_dashboard_active(
    active: bool,
    app: AppHandle,
    runtime: State<'_, Runtime>,
) -> CommandResponse<()> {
    let mut polling = match runtime.0.polling.lock() {
        Ok(polling) => polling,
        Err(error) => {
            tracing::error!(%error, "Dashboard polling state is poisoned");
            return CommandResponse::Failed {
                message: "Dashboard polling is unavailable".to_owned(),
            };
        }
    };

    runtime.0.active.send_if_modified(|current| {
        if *current == active {
            return false;
        }
        *current = active;
        true
    });
    if !active {
        if let Some(task) = polling.take() {
            task.abort();
        }
        return CommandResponse::Ready { data: () };
    }
    if polling
        .as_ref()
        .is_some_and(|task| !task.inner().is_finished())
    {
        return CommandResponse::Ready { data: () };
    }

    let runtime = runtime.inner().clone();
    *polling = Some(tauri::async_runtime::spawn(async move {
        let now = Instant::now();
        let mut task_manager = interval_at(now + Duration::from_secs(2), Duration::from_secs(2));
        let mut subscriptions = interval_at(now + Duration::from_secs(60), Duration::from_secs(60));
        // Steam keeps its polling interval; daily notes only replay their cache.
        let mut games = interval_at(now + Duration::from_secs(300), Duration::from_secs(300));
        loop {
            let sources: &[Source] = tokio::select! {
                _ = games.tick() => &[Source::Games],
                _ = task_manager.tick() => &[Source::TaskManager, Source::DeviceTelemetry],
                _ = subscriptions.tick() => &[
                    Source::Codex,
                    Source::OpenCode,
                    Source::Claude,
                    Source::Grok,
                    Source::Copilot,
                    Source::DeepSeek,
                    Source::CherryIn,
                    Source::TokenFlux,
                    Source::DimAgent,
                    Source::ServiceStatus,
                ],
            };
            let layout = Arc::new(layout());
            for source in sources {
                runtime.poll(&app, &layout, *source);
            }
        }
    }));
    CommandResponse::Ready { data: () }
}

#[tauri::command]
pub(crate) fn read_layout() -> CommandResponse<Layout> {
    layout().into()
}

#[tauri::command]
pub(crate) fn save_layout(layout: Layout, app: AppHandle) -> CommandResponse<()> {
    write(&app, &layout).into()
}

#[tauri::command]
pub(crate) fn reset_layout(app: AppHandle) -> CommandResponse<Layout> {
    let layout = Layout::default();
    write(&app, &layout).map(|()| layout).into()
}

fn write(app: &AppHandle, layout: &Layout) -> Result<(), String> {
    let path = database::path()
        .map_err(|error| format!("Could not resolve Dashboard database: {error}"))?;
    dashboard::write(&path, layout)?;
    crate::island::sync(app);
    Ok(())
}

#[tauri::command]
pub(crate) fn read_service_catalog() -> CommandResponse<Vec<service_status::ServiceCatalogEntry>> {
    CommandResponse::Ready {
        data: service_status::read_catalog(),
    }
}

#[cfg(test)]
#[path = "../tests/unit/dashboard.rs"]
mod tests;
