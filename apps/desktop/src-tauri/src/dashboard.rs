use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinSet;
use tokio::time::{Instant, interval_at};

use crate::{CommandResponse, telemetry, widgets};
use quotes::{exchange, github, quotations, status, stocks, weather};

const EVENT: &str = "dashboard-source-updated";
const SOURCE_COUNT: usize = 15;

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
    ];
}

#[derive(serde::Serialize)]
#[serde(tag = "source", content = "result", rename_all = "camelCase")]
enum DashboardEvent {
    TaskManager(CommandResponse<ugos::TaskManagerSnapshot>),
    DeviceTelemetry(CommandResponse<Option<telemetry::Snapshot>>),
    Codex(CommandResponse<useage::codex::CodexUsage>),
    OpenCode(CommandResponse<useage::opencode::OpenCodeUsage>),
    Claude(CommandResponse<Option<useage::claude::ClaudeUsage>>),
    Grok(CommandResponse<Option<useage::grok::GrokUsage>>),
    Copilot(CommandResponse<Option<useage::copilot::CopilotUsage>>),
    DeepSeek(CommandResponse<useage::deepseek::DeepSeekBalance>),
    CherryIn(CommandResponse<useage::cherryin::CherryInBalance>),
    Weather(Box<CommandResponse<weather::WeatherReport>>),
    Stocks(Box<CommandResponse<stocks::StockReport>>),
    Exchange(Box<CommandResponse<Option<exchange::ExchangeReport>>>),
    ServiceStatus(Box<CommandResponse<status::ServiceStatusReport>>),
    Github(CommandResponse<github::GithubSnapshot>),
    Quotation(CommandResponse<Option<quotations::Quotation>>),
}

impl DashboardEvent {
    async fn read(source: Source, app: &AppHandle) -> Self {
        match source {
            Source::TaskManager => match ugos::task_manager().await {
                Ok(data) => Self::TaskManager(CommandResponse::Ready { data }),
                Err(error) => {
                    tracing::warn!(error = %error, "failed to load UGOS Task Manager");
                    Self::TaskManager(CommandResponse::Failed {
                        message: error.to_string(),
                    })
                }
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
            Source::Codex => match useage::codex::read().await {
                Ok(data) => Self::Codex(CommandResponse::Ready { data }),
                Err(message) => {
                    tracing::warn!(error = %message, "failed to load Codex usage");
                    Self::Codex(CommandResponse::Failed { message })
                }
            },
            Source::OpenCode => match useage::opencode::read().await {
                Ok(data) => Self::OpenCode(CommandResponse::Ready { data }),
                Err(message) => {
                    tracing::warn!(error = %message, "failed to load OpenCode Go usage");
                    Self::OpenCode(CommandResponse::Failed { message })
                }
            },
            Source::Claude => match widgets::has_provider(app, widgets::ProviderWidget::Claude) {
                Ok(false) => Self::Claude(CommandResponse::Ready { data: None }),
                Ok(true) => match useage::claude::read().await {
                    Ok(data) => Self::Claude(CommandResponse::Ready { data: Some(data) }),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load Claude usage");
                        Self::Claude(CommandResponse::Failed { message })
                    }
                },
                Err(message) => Self::Claude(CommandResponse::Failed { message }),
            },
            Source::Grok => match widgets::has_provider(app, widgets::ProviderWidget::Grok) {
                Ok(false) => Self::Grok(CommandResponse::Ready { data: None }),
                Ok(true) => match useage::grok::read().await {
                    Ok(data) => Self::Grok(CommandResponse::Ready { data: Some(data) }),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load Grok usage");
                        Self::Grok(CommandResponse::Failed { message })
                    }
                },
                Err(message) => Self::Grok(CommandResponse::Failed { message }),
            },
            Source::Copilot => match widgets::has_provider(app, widgets::ProviderWidget::Copilot) {
                Ok(false) => Self::Copilot(CommandResponse::Ready { data: None }),
                Ok(true) => match useage::copilot::read().await {
                    Ok(data) => Self::Copilot(CommandResponse::Ready { data: Some(data) }),
                    Err(message) => {
                        tracing::warn!(error = %message, "failed to load Copilot usage");
                        Self::Copilot(CommandResponse::Failed { message })
                    }
                },
                Err(message) => Self::Copilot(CommandResponse::Failed { message }),
            },
            Source::DeepSeek => match useage::deepseek::read().await {
                Ok(data) => Self::DeepSeek(CommandResponse::Ready { data }),
                Err(message) => {
                    tracing::warn!(error = %message, "failed to load DeepSeek balance");
                    Self::DeepSeek(CommandResponse::Failed { message })
                }
            },
            Source::CherryIn => match useage::cherryin::read().await {
                Ok(data) => Self::CherryIn(CommandResponse::Ready { data }),
                Err(message) => {
                    tracing::warn!(error = %message, "failed to load CherryIN balance");
                    Self::CherryIn(CommandResponse::Failed { message })
                }
            },
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
            Source::Github => match github::read().await {
                Ok(data) => Self::Github(CommandResponse::Ready { data }),
                Err(message) => {
                    tracing::warn!(error = %message, "failed to load GitHub activity");
                    Self::Github(CommandResponse::Failed { message })
                }
            },
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

struct RuntimeState {
    sources: [Arc<AsyncMutex<()>>; SOURCE_COUNT],
    polling: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub(crate) struct DashboardRuntime(Arc<RuntimeState>);

impl Default for DashboardRuntime {
    fn default() -> Self {
        Self(Arc::new(RuntimeState {
            sources: std::array::from_fn(|_| Arc::new(AsyncMutex::new(()))),
            polling: Mutex::new(None),
        }))
    }
}

impl DashboardRuntime {
    fn refresh_if_idle(&self, app: AppHandle, source: Source) {
        let Ok(source_guard) = Arc::clone(&self.0.sources[source as usize]).try_lock_owned() else {
            return;
        };
        tauri::async_runtime::spawn(async move {
            let event = DashboardEvent::read(source, &app).await;
            event.emit(&app);
            drop(source_guard);
        });
    }
}

#[tauri::command]
pub(crate) async fn refresh_dashboard(app: AppHandle) -> CommandResponse<()> {
    let runtime = app.state::<DashboardRuntime>().inner().clone();
    let mut requests = JoinSet::new();
    for source in Source::ALL {
        let source_lock = Arc::clone(&runtime.0.sources[source as usize]);
        let request_app = app.clone();
        requests.spawn(async move {
            let source_guard = source_lock.lock_owned().await;
            let event = DashboardEvent::read(source, &request_app).await;
            drop(source_guard);
            event
        });
    }

    while let Some(request) = requests.join_next().await {
        match request {
            Ok(event) => event.emit(&app),
            Err(error) => tracing::error!(%error, "Dashboard source task failed"),
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
        loop {
            tokio::select! {
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
