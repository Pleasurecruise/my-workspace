use std::sync::Arc;
use tauri::{Emitter, Manager};

mod configuration;
mod dashboard;
mod github;
mod weather;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum CommandResponse<T> {
    Ready { data: T },
    Failed { message: String },
}

#[tauri::command]
async fn read_consumer_channel(
    channel: String,
    cursor: Option<String>,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::consumer::ChannelView> {
    let channel = match cms_core::consumer::Channel::try_from(channel.as_str()) {
        Ok(channel) => channel,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let state = app.state::<CmsState>();
    state.channel(channel, cursor).await
}

#[derive(Clone, serde::Serialize)]
struct InitialViews {
    memos: CommandResponse<cms_core::consumer::ChannelView>,
    moment: CommandResponse<cms_core::consumer::ChannelView>,
    knowledge: CommandResponse<cms_core::consumer::ChannelView>,
}

#[tauri::command]
async fn initialize_consumer_views(app: tauri::AppHandle) -> InitialViews {
    app.state::<CmsState>().initial_views().await
}

#[tauri::command]
async fn read_consumer_asset(key: String, app: tauri::AppHandle) -> CommandResponse<Vec<u8>> {
    let state = app.state::<CmsState>();
    let repository = match state.repository().await {
        Ok(repository) => repository,
        Err(message) => return CommandResponse::Failed { message },
    };
    match cms_core::consumer::asset(&key, repository.as_ref()).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn create_consumer_memo(
    content: String,
    visibility: cms_core::api::memos::Visibility,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::create(&content, visibility).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn update_consumer_memo(
    id: String,
    input: cms_core::api::memos::Update,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::update(&id, &input).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn delete_consumer_memo(id: String) -> CommandResponse<String> {
    match cms_core::api::memos::delete(&id).await {
        Ok(()) => CommandResponse::Ready { data: id },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
fn compile_knowledge(source: String) -> CommandResponse<cms_core::markdown::CompiledKnowledge> {
    CommandResponse::Ready {
        data: cms_core::markdown::compile_knowledge(&source),
    }
}

#[tauri::command]
async fn read_todos(date: String, app: tauri::AppHandle) -> CommandResponse<cms_core::todo::List> {
    match app.state::<cms_core::todo::Store>().list(&date).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn add_todo(
    date: String,
    text: String,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::todo::List> {
    match app
        .state::<cms_core::todo::Store>()
        .create(&date, &text)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn set_todo_completed(
    date: String,
    id: String,
    completed: bool,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::todo::List> {
    match app
        .state::<cms_core::todo::Store>()
        .set_completed(&date, &id, completed)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn delete_todo(
    date: String,
    id: String,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::todo::List> {
    match app
        .state::<cms_core::todo::Store>()
        .delete(&date, &id)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

pub(crate) struct CmsState {
    repository: tokio::sync::Mutex<Option<Arc<cms_core::consumer::Repository>>>,
    initial_views: tokio::sync::Mutex<Option<InitialViews>>,
}

impl Default for CmsState {
    fn default() -> Self {
        Self {
            repository: tokio::sync::Mutex::new(None),
            initial_views: tokio::sync::Mutex::new(None),
        }
    }
}

impl CmsState {
    async fn repository(&self) -> Result<Arc<cms_core::consumer::Repository>, String> {
        let mut state = self.repository.lock().await;
        if let Some(repository) = state.as_ref() {
            return Ok(Arc::clone(repository));
        }
        let repository = cms_core::r2::Store::from_credentials()
            .await
            .map(cms_core::consumer::Repository::new)
            .map(Arc::new)
            .map_err(|error| error.to_string())?;
        *state = Some(Arc::clone(&repository));
        Ok(repository)
    }

    async fn initial_views(&self) -> InitialViews {
        let mut state = self.initial_views.lock().await;
        if let Some(views) = state.as_ref() {
            return views.clone();
        }
        let (memos, moment, knowledge) = tokio::join!(
            self.channel(cms_core::consumer::Channel::Memos, None),
            self.channel(cms_core::consumer::Channel::Moment, None),
            self.channel(cms_core::consumer::Channel::Knowledge, None),
        );
        let views = InitialViews {
            memos,
            moment,
            knowledge,
        };
        *state = Some(views.clone());
        views
    }

    pub(crate) async fn reset(&self) {
        self.reset_views().await;
        *self.repository.lock().await = None;
    }

    pub(crate) async fn reset_views(&self) {
        *self.initial_views.lock().await = None;
    }

    async fn channel(
        &self,
        channel: cms_core::consumer::Channel,
        cursor: Option<String>,
    ) -> CommandResponse<cms_core::consumer::ChannelView> {
        let result = match channel {
            cms_core::consumer::Channel::Memos => {
                let repository = match self.repository().await {
                    Ok(repository) => repository,
                    Err(message) => return CommandResponse::Failed { message },
                };
                match cms_core::api::memos::list(repository.store(), cursor).await {
                    Ok(page) => Ok(cms_core::consumer::ChannelView::Memos {
                        connected: true,
                        memos: page.memos,
                        next_cursor: page.next_cursor,
                    }),
                    Err(error) => Err(error.to_string()),
                }
            }
            cms_core::consumer::Channel::Knowledge => {
                match cms_core::api::knowledge::list(cursor).await {
                    Ok(page) => Ok(cms_core::consumer::ChannelView::Knowledge {
                        connected: true,
                        knowledge: page.documents,
                        next_cursor: page.cursor,
                    }),
                    Err(error) => Err(error.to_string()),
                }
            }
            cms_core::consumer::Channel::Moment => {
                match cms_core::api::moment::list(cursor).await {
                    Ok(page) => Ok(cms_core::consumer::ChannelView::Moment {
                        connected: true,
                        photos: page.photos,
                        total: page.total,
                        next_cursor: page.next_cursor,
                    }),
                    Err(error) => Err(error.to_string()),
                }
            }
        };
        match result {
            Ok(data) => CommandResponse::Ready { data },
            Err(message) => CommandResponse::Failed { message },
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    if let Err(error) = vesper_credentials::load_development_environment() {
        panic!("failed to load development credentials: {error}");
    }
    if let Err(error) = my_workspace_logger::init() {
        panic!("failed to initialize logging: {error}");
    }
    my_workspace_logger::info!("starting desktop application");

    let result = tauri::Builder::default()
        .manage(CmsState::default())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let todo_path = app.path().app_data_dir()?.join("todos.json");
            app.manage(cms_core::todo::Store::new(todo_path));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let delay = match cms_core::todo::next_rollover_delay() {
                        Ok(delay) => delay,
                        Err(error) => {
                            tracing::error!(%error, "failed to schedule Todo rollover");
                            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                            continue;
                        }
                    };
                    tokio::time::sleep(delay).await;
                    let date = match cms_core::todo::current_date() {
                        Ok(date) => date,
                        Err(error) => {
                            tracing::error!(%error, "failed to resolve the date for Todo rollover");
                            continue;
                        }
                    };
                    match handle.state::<cms_core::todo::Store>().list(&date).await {
                        Ok(list) => {
                            if let Err(error) = handle.emit("todo-list-changed", list) {
                                tracing::warn!(%error, "failed to notify the Todo view after rollover");
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, "failed to load the new Todo date at midnight");
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initialize_consumer_views,
            read_consumer_channel,
            read_consumer_asset,
            create_consumer_memo,
            update_consumer_memo,
            delete_consumer_memo,
            compile_knowledge,
            dashboard::read_task_manager,
            dashboard::read_codex_usage,
            dashboard::read_opencode_usage,
            dashboard::read_deepseek_balance,
            dashboard::read_cherryin_balance,
            dashboard::read_weather,
            dashboard::read_github,
            read_todos,
            add_todo,
            set_todo_completed,
            delete_todo,
            configuration::read_configuration,
            configuration::save_ugos_configuration,
            configuration::save_r2_configuration,
            configuration::save_api_configuration
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        panic!("error while running tauri application: {error}");
    }
}
