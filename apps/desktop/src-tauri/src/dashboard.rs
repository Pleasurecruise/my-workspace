use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex as AsyncMutex, watch};
use tokio::task::JoinSet;
use tokio::time::{Instant, interval_at};

use crate::{CommandResponse, telemetry, widgets};
use quotes::{exchange, github, quotations, status, stocks, weather};

const EVENT: &str = "dashboard-source-updated";
const SOURCE_COUNT: usize = 16;

#[derive(Clone, Copy)]
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
    Weather(Box<CommandResponse<weather::WeatherReport>>),
    Stocks(Box<CommandResponse<stocks::StockReport>>),
    Exchange(Box<CommandResponse<Option<exchange::ExchangeReport>>>),
    ServiceStatus(Box<CommandResponse<status::ServiceStatusReport>>),
    Github(CommandResponse<Option<github::GithubSnapshot>>),
    Quotation(CommandResponse<Option<quotations::Quotation>>),
    Games(CommandResponse<()>),
}

impl DashboardEvent {
    async fn read(source: Source, app: &AppHandle, refresh_games: bool) -> Self {
        match source {
            Source::Games => match crate::gaming::refresh(app, refresh_games).await {
                Ok(data) => Self::Games(CommandResponse::Ready { data }),
                Err(message) => Self::Games(CommandResponse::Failed { message }),
            },
            Source::TaskManager => match widgets::has_ugos(app) {
                Ok(false) => Self::TaskManager(CommandResponse::Ready { data: None }),
                Ok(true) => match ugos::task_manager().await {
                    Ok(data) => Self::TaskManager(CommandResponse::Ready { data: Some(data) }),
                    Err(error) => {
                        tracing::warn!(error = %error, "failed to load UGOS Task Manager");
                        Self::TaskManager(CommandResponse::Failed {
                            message: error.to_string(),
                        })
                    }
                },
                Err(message) => Self::TaskManager(CommandResponse::Failed { message }),
            },
            Source::DeviceTelemetry => match widgets::has_device_telemetry(app) {
                Ok(false) => Self::DeviceTelemetry(CommandResponse::Ready { data: None }),
                Ok(true) => match telemetry::read().await {
                    Ok(data) => Self::DeviceTelemetry(CommandResponse::Ready { data: Some(data) }),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to read current-device telemetry");
                        Self::DeviceTelemetry(CommandResponse::Failed { message })
                    }
                },
                Err(message) => Self::DeviceTelemetry(CommandResponse::Failed { message }),
            },
            Source::Codex => Self::Codex(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::Codex),
                    useage::codex::read(),
                )
                .await,
            ),
            Source::OpenCode => Self::OpenCode(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::OpenCode),
                    useage::opencode::read(),
                )
                .await,
            ),
            Source::Claude => Self::Claude(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::Claude),
                    useage::claude::read(),
                )
                .await,
            ),
            Source::Grok => Self::Grok(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::Grok),
                    useage::grok::read(),
                )
                .await,
            ),
            Source::Copilot => Self::Copilot(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::Copilot),
                    useage::copilot::read(),
                )
                .await,
            ),
            Source::DeepSeek => Self::DeepSeek(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::DeepSeek),
                    useage::deepseek::read(),
                )
                .await,
            ),
            Source::CherryIn => Self::CherryIn(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::CherryIn),
                    useage::cherryin::read(),
                )
                .await,
            ),
            Source::Weather => match widgets::weather_locations(app) {
                Ok(locations) => match weather::read(locations).await {
                    Ok(data) => Self::Weather(Box::new(CommandResponse::Ready { data })),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load weather");
                        Self::Weather(Box::new(CommandResponse::Failed { message }))
                    }
                },
                Err(message) => Self::Weather(Box::new(CommandResponse::Failed { message })),
            },
            Source::Stocks => match widgets::stock_symbols(app) {
                Ok(symbols) => match stocks::read(symbols).await {
                    Ok(data) => Self::Stocks(Box::new(CommandResponse::Ready { data })),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load stocks");
                        Self::Stocks(Box::new(CommandResponse::Failed { message }))
                    }
                },
                Err(message) => Self::Stocks(Box::new(CommandResponse::Failed { message })),
            },
            Source::Exchange => match widgets::has_exchange(app) {
                Ok(false) => Self::Exchange(Box::new(CommandResponse::Ready { data: None })),
                Ok(true) => match exchange::read().await {
                    Ok(data) => {
                        Self::Exchange(Box::new(CommandResponse::Ready { data: Some(data) }))
                    }
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load exchange rates");
                        Self::Exchange(Box::new(CommandResponse::Failed { message }))
                    }
                },
                Err(message) => Self::Exchange(Box::new(CommandResponse::Failed { message })),
            },
            Source::ServiceStatus => match widgets::service_status_ids(app) {
                Ok(service_ids) => match status::read(service_ids).await {
                    Ok(data) => Self::ServiceStatus(Box::new(CommandResponse::Ready { data })),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load service status");
                        Self::ServiceStatus(Box::new(CommandResponse::Failed { message }))
                    }
                },
                Err(message) => Self::ServiceStatus(Box::new(CommandResponse::Failed { message })),
            },
            Source::Github => Self::Github(
                read_provider(
                    widgets::has_provider(app, widgets::ProviderWidget::Github),
                    github::read(),
                )
                .await,
            ),
            Source::Quotation => match widgets::has_quotation(app) {
                Ok(false) => Self::Quotation(CommandResponse::Ready { data: None }),
                Ok(true) => match quotations::read().await {
                    Ok(data) => Self::Quotation(CommandResponse::Ready { data: Some(data) }),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load random quotation");
                        Self::Quotation(CommandResponse::Failed { message })
                    }
                },
                Err(message) => Self::Quotation(CommandResponse::Failed { message }),
            },
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

// A removed widget must not read credentials, start a CLI, or renew an OAuth session.
async fn read_provider<T>(
    enabled: Result<bool, String>,
    read: impl std::future::Future<Output = Result<T, String>>,
) -> CommandResponse<Option<T>> {
    let result = match enabled {
        Ok(false) => return CommandResponse::Ready { data: None },
        Ok(true) => read.await,
        Err(message) => return CommandResponse::Failed { message },
    };
    match result {
        Ok(data) => CommandResponse::Ready { data: Some(data) },
        Err(message) => {
            tracing::warn!(error = %message, "Dashboard provider unavailable");
            CommandResponse::Failed { message }
        }
    }
}

struct RuntimeState {
    active: watch::Sender<bool>,
    sources: [Arc<AsyncMutex<()>>; SOURCE_COUNT],
    polling: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub(crate) struct DashboardRuntime(Arc<RuntimeState>);

impl Default for DashboardRuntime {
    fn default() -> Self {
        Self(Arc::new(RuntimeState {
            active: watch::channel(false).0,
            sources: std::array::from_fn(|_| Arc::new(AsyncMutex::new(()))),
            polling: Mutex::new(None),
        }))
    }
}

impl DashboardRuntime {
    fn refresh_if_idle(&self, app: AppHandle, source: Source) {
        let active = self.0.active.subscribe();
        let Ok(source_guard) = Arc::clone(&self.0.sources[source as usize]).try_lock_owned() else {
            return;
        };
        tauri::async_runtime::spawn(async move {
            if let Some(event) =
                read_while_active(active, DashboardEvent::read(source, &app, false)).await
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

fn island_source(widget: &widgets::Widget) -> Option<Source> {
    use widgets::Widget;
    Some(match widget {
        Widget::Cpu | Widget::Memory | Widget::Storage | Widget::Network => Source::TaskManager,
        Widget::LocalCpu | Widget::LocalMemory | Widget::LocalStorage | Widget::LocalNetwork => {
            Source::DeviceTelemetry
        }
        Widget::Weather { .. } => Source::Weather,
        Widget::Stock { .. } => Source::Stocks,
        Widget::Exchange => Source::Exchange,
        Widget::ServiceStatus { .. } => Source::ServiceStatus,
        Widget::Github => Source::Github,
        Widget::Codex => Source::Codex,
        Widget::OpenCode => Source::OpenCode,
        Widget::Claude => Source::Claude,
        Widget::Grok => Source::Grok,
        Widget::Copilot => Source::Copilot,
        Widget::DeepSeek => Source::DeepSeek,
        Widget::CherryIn => Source::CherryIn,
        Widget::Quotation => Source::Quotation,
        // Todo is read through its own session; game panels own their initial read.
        Widget::Invalid { .. }
        | Widget::Planner { .. }
        | Widget::Spending
        | Widget::Game { .. }
        | Widget::Steam => {
            return None;
        }
    })
}

#[tauri::command]
pub(crate) async fn refresh_island(app: AppHandle) -> CommandResponse<()> {
    let widget = match widgets::island_widget(&app) {
        Ok(Some(widget)) => widget,
        Ok(None) => return CommandResponse::Ready { data: () },
        Err(message) => return CommandResponse::Failed { message },
    };
    if let Some(source) = island_source(&widget) {
        let runtime = app.state::<DashboardRuntime>();
        let _guard = runtime.0.sources[source as usize].lock().await;
        DashboardEvent::read(source, &app, false).await.emit(&app);
    }
    CommandResponse::Ready { data: () }
}

#[tauri::command]
pub(crate) async fn refresh_dashboard(
    app: AppHandle,
    refresh_games: Option<bool>,
) -> CommandResponse<()> {
    let runtime = app.state::<DashboardRuntime>().inner().clone();
    let mut active = runtime.0.active.subscribe();
    if !*active.borrow_and_update() {
        return CommandResponse::Ready { data: () };
    }
    let mut requests = JoinSet::new();
    for source in Source::ALL {
        let source_lock = Arc::clone(&runtime.0.sources[source as usize]);
        let request_app = app.clone();
        let source_active = active.clone();
        requests.spawn(async move {
            read_while_active(source_active, async {
                let source_guard = source_lock.lock_owned().await;
                let event =
                    DashboardEvent::read(source, &request_app, refresh_games.unwrap_or(false))
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
    runtime: State<'_, DashboardRuntime>,
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
            tokio::select! {
                _ = games.tick() => runtime.refresh_if_idle(app.clone(), Source::Games),
                _ = task_manager.tick() => {
                    runtime.refresh_if_idle(app.clone(), Source::TaskManager);
                    runtime.refresh_if_idle(app.clone(), Source::DeviceTelemetry);
                },
                _ = subscriptions.tick() => {
                    runtime.refresh_if_idle(app.clone(), Source::Codex);
                    runtime.refresh_if_idle(app.clone(), Source::OpenCode);
                    runtime.refresh_if_idle(app.clone(), Source::Claude);
                    runtime.refresh_if_idle(app.clone(), Source::Grok);
                    runtime.refresh_if_idle(app.clone(), Source::Copilot);
                    runtime.refresh_if_idle(app.clone(), Source::DeepSeek);
                    runtime.refresh_if_idle(app.clone(), Source::CherryIn);
                    runtime.refresh_if_idle(app.clone(), Source::ServiceStatus);
                }
            }
        }
    }));
    CommandResponse::Ready { data: () }
}

#[cfg(test)]
#[path = "../tests/unit/dashboard.rs"]
mod tests;
