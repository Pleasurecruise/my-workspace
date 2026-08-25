use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

mod configuration;
mod dashboard;
mod github;
mod notifications;
mod updater;
mod weather;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum CommandResponse<T> {
    Ready { data: T },
    Failed { message: String },
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelQuery {
    channel: String,
    cursor: Option<String>,
    search: Option<String>,
    tags: Vec<String>,
    sort_by_updated: bool,
    archived_only: bool,
    favorites_only: bool,
}

struct ChannelRequest {
    channel: cms_core::consumer::Channel,
    cursor: Option<String>,
    filters: cms_core::api::memos::ListFilters,
    read_cached_first_page: bool,
}

impl ChannelRequest {
    fn initial(channel: cms_core::consumer::Channel) -> Self {
        Self {
            channel,
            cursor: None,
            filters: cms_core::api::memos::ListFilters::default(),
            read_cached_first_page: true,
        }
    }
}

const ASSET_LIMIT: usize = 64;
const ASSET_BYTES: usize = 128 * 1024 * 1024;
const VIEW_TTL: Duration = Duration::from_secs(30);

struct CachedView {
    data: cms_core::consumer::ChannelView,
    loaded_at: Instant,
}

#[derive(Default)]
struct ViewCache {
    memos: Option<CachedView>,
    moment: Option<CachedView>,
    knowledge: Option<CachedView>,
}

impl ViewCache {
    fn get(&self, channel: cms_core::consumer::Channel) -> Option<cms_core::consumer::ChannelView> {
        let entry = match channel {
            cms_core::consumer::Channel::Memos => self.memos.as_ref(),
            cms_core::consumer::Channel::Moment => self.moment.as_ref(),
            cms_core::consumer::Channel::Knowledge => self.knowledge.as_ref(),
        }?;
        (entry.loaded_at.elapsed() <= VIEW_TTL).then(|| entry.data.clone())
    }

    fn insert(
        &mut self,
        channel: cms_core::consumer::Channel,
        data: cms_core::consumer::ChannelView,
    ) {
        let entry = Some(CachedView {
            data,
            loaded_at: Instant::now(),
        });
        match channel {
            cms_core::consumer::Channel::Memos => self.memos = entry,
            cms_core::consumer::Channel::Moment => self.moment = entry,
            cms_core::consumer::Channel::Knowledge => self.knowledge = entry,
        }
    }

    fn clear(&mut self, channel: cms_core::consumer::Channel) {
        match channel {
            cms_core::consumer::Channel::Memos => self.memos = None,
            cms_core::consumer::Channel::Moment => self.moment = None,
            cms_core::consumer::Channel::Knowledge => self.knowledge = None,
        }
    }
}

#[derive(Default)]
struct AssetCache {
    data: HashMap<String, Vec<u8>>,
    order: VecDeque<String>,
    bytes: usize,
}

impl AssetCache {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        let data = self.data.get(key)?.clone();
        self.order.retain(|cached| cached != key);
        self.order.push_back(key.to_owned());
        Some(data)
    }

    fn insert(&mut self, key: String, data: Vec<u8>) {
        if data.len() > ASSET_BYTES {
            return;
        }
        if let Some(previous) = self.data.remove(&key) {
            self.bytes -= previous.len();
            self.order.retain(|cached| cached != &key);
        }
        self.bytes += data.len();
        self.order.push_back(key.clone());
        self.data.insert(key, data);
        while self.order.len() > ASSET_LIMIT || self.bytes > ASSET_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(removed) = self.data.remove(&oldest) {
                self.bytes -= removed.len();
            }
        }
    }

    fn clear(&mut self) {
        self.data.clear();
        self.order.clear();
        self.bytes = 0;
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn view_cache_expires_and_clears_first_pages() {
        let mut cache = ViewCache::default();
        cache.insert(
            cms_core::consumer::Channel::Moment,
            cms_core::consumer::ChannelView::Moment {
                connected: true,
                photos: Vec::new(),
                tags: Vec::new(),
                total: 0,
                next_cursor: Some("cursor".to_owned()),
            },
        );
        assert!(cache.get(cms_core::consumer::Channel::Moment).is_some());
        cache.moment.as_mut().unwrap().loaded_at =
            Instant::now() - VIEW_TTL - Duration::from_secs(1);
        assert!(cache.get(cms_core::consumer::Channel::Moment).is_none());
        cache.clear(cms_core::consumer::Channel::Moment);
        assert!(cache.moment.is_none());
    }

    #[test]
    fn asset_cache_evicts_the_oldest_object() {
        let mut cache = AssetCache::default();
        for index in 0..=ASSET_LIMIT {
            cache.insert(format!("img/{index}.jpg"), vec![index as u8]);
        }
        assert!(cache.get("img/0.jpg").is_none());
        assert_eq!(
            cache.get(&format!("img/{ASSET_LIMIT}.jpg")),
            Some(vec![ASSET_LIMIT as u8])
        );
        cache.clear();
        assert!(cache.data.is_empty());
        assert_eq!(cache.bytes, 0);
    }

    #[tokio::test]
    #[ignore = "requires live consumer and R2 credentials"]
    async fn live_consumer_pages_advance_without_using_the_first_page_cache() {
        vesper_credentials::load_development_environment()
            .expect("development credentials should load");
        let state = CmsState::default();

        for channel in [
            cms_core::consumer::Channel::Memos,
            cms_core::consumer::Channel::Moment,
        ] {
            let channel_name = match channel {
                cms_core::consumer::Channel::Memos => "memos",
                cms_core::consumer::Channel::Moment => "moment",
                cms_core::consumer::Channel::Knowledge => "knowledge",
            };
            let first = state
                .channel(ChannelRequest {
                    channel,
                    cursor: None,
                    filters: cms_core::api::memos::ListFilters::default(),
                    read_cached_first_page: false,
                })
                .await;
            let cursor = match first {
                CommandResponse::Ready {
                    data: cms_core::consumer::ChannelView::Memos { next_cursor, .. },
                }
                | CommandResponse::Ready {
                    data: cms_core::consumer::ChannelView::Moment { next_cursor, .. },
                } => next_cursor.expect("the live first page should have a cursor"),
                CommandResponse::Ready { .. } => panic!("channel returned the wrong view"),
                CommandResponse::Failed { message } => {
                    panic!("{channel_name} first page failed: {message}")
                }
            };

            let second = state
                .channel(ChannelRequest {
                    channel,
                    cursor: Some(cursor),
                    filters: cms_core::api::memos::ListFilters::default(),
                    read_cached_first_page: false,
                })
                .await;
            match second {
                CommandResponse::Ready {
                    data: cms_core::consumer::ChannelView::Memos { memos, tags, .. },
                } => {
                    assert!(!memos.is_empty());
                    assert!(tags.is_empty());
                }
                CommandResponse::Ready {
                    data: cms_core::consumer::ChannelView::Moment { photos, tags, .. },
                } => {
                    assert!(!photos.is_empty());
                    assert!(tags.is_empty());
                }
                CommandResponse::Ready { .. } => panic!("channel returned the wrong view"),
                CommandResponse::Failed { message } => {
                    panic!("{channel_name} second page failed: {message}")
                }
            }
        }
    }
}

#[tauri::command]
async fn read_channel(
    query: ChannelQuery,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::consumer::ChannelView> {
    let paginated = query.cursor.is_some();
    let started = Instant::now();
    let channel = match cms_core::consumer::Channel::try_from(query.channel.as_str()) {
        Ok(channel) => channel,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let state = app.state::<CmsState>();
    let response = state
        .channel(ChannelRequest {
            channel,
            cursor: query.cursor,
            filters: cms_core::api::memos::ListFilters {
                limit: None,
                search: query.search,
                tags: query.tags,
                sort_by_updated: query.sort_by_updated,
                archived_only: query.archived_only,
                favorites_only: query.favorites_only,
            },
            read_cached_first_page: false,
        })
        .await;
    match &response {
        CommandResponse::Ready { data } => {
            let (items, has_next) = match data {
                cms_core::consumer::ChannelView::Memos {
                    memos, next_cursor, ..
                } => (memos.len(), next_cursor.is_some()),
                cms_core::consumer::ChannelView::Moment {
                    photos,
                    next_cursor,
                    ..
                } => (photos.len(), next_cursor.is_some()),
                cms_core::consumer::ChannelView::Knowledge {
                    knowledge,
                    next_cursor,
                    ..
                } => (knowledge.len(), next_cursor.is_some()),
            };
            tracing::info!(
                channel = ?channel,
                paginated,
                items,
                has_next,
                elapsed_ms = started.elapsed().as_millis(),
                "consumer channel ready"
            );
        }
        CommandResponse::Failed { message } => {
            tracing::warn!(
                channel = ?channel,
                paginated,
                error = %message,
                elapsed_ms = started.elapsed().as_millis(),
                "consumer channel failed"
            );
        }
    }
    response
}

#[tauri::command]
async fn read_memo_tags() -> CommandResponse<Vec<cms_core::api::memos::TagCount>> {
    match cms_core::api::memos::tags().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn read_moment_tags() -> CommandResponse<Vec<String>> {
    match cms_core::api::moment::tags().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[derive(Clone, serde::Serialize)]
struct InitialViews {
    memos: CommandResponse<cms_core::consumer::ChannelView>,
    moment: CommandResponse<cms_core::consumer::ChannelView>,
    knowledge: CommandResponse<cms_core::consumer::ChannelView>,
}

#[tauri::command]
async fn initialize_views(app: tauri::AppHandle) -> InitialViews {
    app.state::<CmsState>().initial_views().await
}

#[tauri::command]
async fn read_asset(key: String, app: tauri::AppHandle) -> CommandResponse<Vec<u8>> {
    let state = app.state::<CmsState>();
    if let Some(data) = state.assets.lock().await.get(&key) {
        return CommandResponse::Ready { data };
    }
    let repository = match state.repository().await {
        Ok(repository) => repository,
        Err(message) => return CommandResponse::Failed { message },
    };
    match cms_core::consumer::asset(&key, repository.as_ref()).await {
        Ok(data) => {
            state.assets.lock().await.insert(key, data.clone());
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn create_memo(
    content: String,
    visibility: cms_core::api::memos::Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::create(&content, visibility).await {
        Ok(data) => {
            app.state::<CmsState>()
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Memos);
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn import_x_memo(
    url: String,
    visibility: cms_core::api::memos::Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::import_x(&url, visibility).await {
        Ok(data) => {
            app.state::<CmsState>()
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Memos);
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn update_memo(
    id: String,
    input: cms_core::api::memos::Update,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::update(&id, &input).await {
        Ok(data) => {
            app.state::<CmsState>()
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Memos);
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn delete_memo(id: String, app: tauri::AppHandle) -> CommandResponse<String> {
    match cms_core::api::memos::delete(&id).await {
        Ok(()) => {
            app.state::<CmsState>()
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Memos);
            CommandResponse::Ready { data: id }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn create_photo(
    input: cms_core::api::moment::Upload,
    original: Vec<u8>,
    thumbnail: Vec<u8>,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::moment::Photo> {
    let state = app.state::<CmsState>();
    let repository = match state.repository().await {
        Ok(repository) => repository,
        Err(message) => return CommandResponse::Failed { message },
    };
    match cms_core::api::moment::upload(repository.store(), input, original, thumbnail).await {
        Ok(data) => {
            state
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Moment);
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn update_photo(
    id: String,
    input: cms_core::api::moment::Update,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::moment::Photo> {
    match cms_core::api::moment::update(&id, &input).await {
        Ok(data) => {
            app.state::<CmsState>()
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Moment);
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn delete_photo(id: String, app: tauri::AppHandle) -> CommandResponse<String> {
    match cms_core::api::moment::delete(&id).await {
        Ok(()) => {
            let state = app.state::<CmsState>();
            state
                .views
                .lock()
                .await
                .clear(cms_core::consumer::Channel::Moment);
            state.assets.lock().await.clear();
            CommandResponse::Ready { data: id }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn create_knowledge(
    input: cms_core::api::knowledge::Draft,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::knowledge::Document> {
    let input = cms_core::api::knowledge::Create::Draft(input);
    match cms_core::api::knowledge::create(&input).await {
        Ok(article) => match cms_core::api::knowledge::project_article(article) {
            Ok(data) => {
                app.state::<CmsState>()
                    .views
                    .lock()
                    .await
                    .clear(cms_core::consumer::Channel::Knowledge);
                CommandResponse::Ready { data }
            }
            Err(error) => CommandResponse::Failed {
                message: error.to_string(),
            },
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
async fn update_knowledge(
    id: String,
    input: cms_core::api::knowledge::DraftUpdate,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::knowledge::Document> {
    match cms_core::api::knowledge::update_draft(&id, &input).await {
        Ok(article) => match cms_core::api::knowledge::project_article(article) {
            Ok(data) => {
                app.state::<CmsState>()
                    .views
                    .lock()
                    .await
                    .clear(cms_core::consumer::Channel::Knowledge);
                CommandResponse::Ready { data }
            }
            Err(error) => CommandResponse::Failed {
                message: error.to_string(),
            },
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
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
    views: tokio::sync::Mutex<ViewCache>,
    assets: tokio::sync::Mutex<AssetCache>,
}

impl Default for CmsState {
    fn default() -> Self {
        Self {
            repository: tokio::sync::Mutex::new(None),
            views: tokio::sync::Mutex::new(ViewCache::default()),
            assets: tokio::sync::Mutex::new(AssetCache::default()),
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
        let (memos, moment, knowledge) = tokio::join!(
            self.channel(ChannelRequest::initial(cms_core::consumer::Channel::Memos)),
            self.channel(ChannelRequest::initial(cms_core::consumer::Channel::Moment)),
            self.channel(ChannelRequest::initial(
                cms_core::consumer::Channel::Knowledge,
            )),
        );
        InitialViews {
            memos,
            moment,
            knowledge,
        }
    }

    pub(crate) async fn reset(&self) {
        self.reset_views().await;
        self.assets.lock().await.clear();
        *self.repository.lock().await = None;
    }

    pub(crate) async fn reset_views(&self) {
        *self.views.lock().await = ViewCache::default();
    }

    async fn channel(
        &self,
        request: ChannelRequest,
    ) -> CommandResponse<cms_core::consumer::ChannelView> {
        let ChannelRequest {
            channel,
            cursor,
            filters,
            read_cached_first_page,
        } = request;
        let cacheable = cursor.is_none()
            && filters.search.is_none()
            && filters.tags.is_empty()
            && !filters.sort_by_updated
            && !filters.archived_only
            && !filters.favorites_only;
        if read_cached_first_page
            && cacheable
            && let Some(data) = self.views.lock().await.get(channel)
        {
            return CommandResponse::Ready { data };
        }
        let result = match channel {
            cms_core::consumer::Channel::Memos => {
                match cms_core::api::memos::list(cursor, &filters).await {
                    Ok(page) => Ok(cms_core::consumer::ChannelView::Memos {
                        connected: true,
                        memos: page.memos,
                        tags: Vec::new(),
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
                        tags: Vec::new(),
                        total: page.total,
                        next_cursor: page.next_cursor,
                    }),
                    Err(error) => Err(error.to_string()),
                }
            }
        };
        match result {
            Ok(data) => {
                if cacheable {
                    self.views.lock().await.insert(channel, data.clone());
                }
                CommandResponse::Ready { data }
            }
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
        .manage(updater::UpdateState::default())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let todo_path = app.path().app_data_dir()?.join("todos.json");
            app.manage(cms_core::todo::Store::new(todo_path));
            let notifications_path = app.path().app_data_dir()?.join("notifications.json");
            app.manage(
                notifications::NotificationState::new(notifications_path)
                    .map_err(std::io::Error::other)?,
            );
            let notifications_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = notifications::restart(notifications_app).await {
                    tracing::warn!(%error, "could not start ntfy notification subscription");
                }
            });
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
            initialize_views,
            read_channel,
            read_memo_tags,
            read_moment_tags,
            read_asset,
            create_memo,
            import_x_memo,
            update_memo,
            delete_memo,
            create_photo,
            update_photo,
            delete_photo,
            create_knowledge,
            update_knowledge,
            updater::check_for_update,
            updater::install_update,
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
            configuration::save_api_configuration,
            configuration::save_ntfy_configuration,
            notifications::read_notifications,
            configuration::save_app_lock,
            configuration::remove_app_lock,
            configuration::unlock_app
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        panic!("error while running tauri application: {error}");
    }
}
