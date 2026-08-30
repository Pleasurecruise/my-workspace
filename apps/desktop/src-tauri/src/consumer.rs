use crate::CommandResponse;
use crate::cms::{ChannelRequest, CmsState};
use std::time::Instant;
use tauri::Manager;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelQuery {
    channel: String,
    cursor: Option<String>,
    search: Option<String>,
    tags: Vec<String>,
    sort_by_updated: bool,
    archived_only: bool,
    favorites_only: bool,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct InitialViews {
    memos: CommandResponse<cms_core::consumer::ChannelView>,
    moment: CommandResponse<cms_core::consumer::ChannelView>,
    knowledge: CommandResponse<cms_core::consumer::ChannelView>,
}

#[tauri::command]
pub(crate) async fn initialize_views(app: tauri::AppHandle) -> InitialViews {
    let state = app.state::<CmsState>();
    let (memos, moment, knowledge) = tokio::join!(
        state.channel(ChannelRequest::initial(cms_core::consumer::Channel::Memos,)),
        state.channel(ChannelRequest::initial(cms_core::consumer::Channel::Moment,)),
        state.channel(ChannelRequest::initial(
            cms_core::consumer::Channel::Knowledge,
        )),
    );
    InitialViews {
        memos,
        moment,
        knowledge,
    }
}

#[tauri::command]
pub(crate) async fn read_channel(
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
    let response = app
        .state::<CmsState>()
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
pub(crate) async fn read_memo_tags() -> CommandResponse<Vec<cms_core::api::memos::TagCount>> {
    match cms_core::api::memos::tags().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn read_moment_tags() -> CommandResponse<Vec<String>> {
    match cms_core::api::moment::tags().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn read_asset(key: String, app: tauri::AppHandle) -> CommandResponse<Vec<u8>> {
    let state = app.state::<CmsState>();
    if let Some(data) = state.cached_asset(&key).await {
        return CommandResponse::Ready { data };
    }
    let repository = match state.repository().await {
        Ok(repository) => repository,
        Err(message) => return CommandResponse::Failed { message },
    };
    match cms_core::consumer::asset(&key, repository.as_ref()).await {
        Ok(data) => {
            state.cache_asset(key, data.clone()).await;
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn create_memo(
    content: String,
    visibility: cms_core::api::memos::Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::create(&content, visibility).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(cms_core::consumer::Channel::Memos)
                .await;
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn import_x_memo(
    url: String,
    visibility: cms_core::api::memos::Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::import_x(&url, visibility).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(cms_core::consumer::Channel::Memos)
                .await;
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn update_memo(
    id: String,
    input: cms_core::api::memos::Update,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::memos::MemoView> {
    match cms_core::api::memos::update(&id, &input).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(cms_core::consumer::Channel::Memos)
                .await;
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn delete_memo(id: String, app: tauri::AppHandle) -> CommandResponse<String> {
    match cms_core::api::memos::delete(&id).await {
        Ok(()) => {
            app.state::<CmsState>()
                .invalidate_view(cms_core::consumer::Channel::Memos)
                .await;
            CommandResponse::Ready { data: id }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn create_photo(
    input: cms_core::api::moment::Upload,
    source: Vec<u8>,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::moment::Photo> {
    let state = app.state::<CmsState>();
    let repository = match state.repository().await {
        Ok(repository) => repository,
        Err(message) => return CommandResponse::Failed { message },
    };
    match cms_core::api::moment::upload(repository.store(), input, source).await {
        Ok(data) => {
            state
                .invalidate_view(cms_core::consumer::Channel::Moment)
                .await;
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn update_photo(
    id: String,
    input: cms_core::api::moment::Update,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::moment::Photo> {
    match cms_core::api::moment::update(&id, &input).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(cms_core::consumer::Channel::Moment)
                .await;
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn delete_photo(id: String, app: tauri::AppHandle) -> CommandResponse<String> {
    match cms_core::api::moment::delete(&id).await {
        Ok(()) => {
            let state = app.state::<CmsState>();
            state
                .invalidate_view(cms_core::consumer::Channel::Moment)
                .await;
            state.clear_assets().await;
            CommandResponse::Ready { data: id }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn create_knowledge(
    input: cms_core::api::knowledge::Draft,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::knowledge::Document> {
    let input = cms_core::api::knowledge::Create::Draft(input);
    match cms_core::api::knowledge::create(&input).await {
        Ok(article) => match cms_core::api::knowledge::project_article(article) {
            Ok(data) => {
                app.state::<CmsState>()
                    .invalidate_view(cms_core::consumer::Channel::Knowledge)
                    .await;
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
pub(crate) async fn update_knowledge(
    id: String,
    input: cms_core::api::knowledge::DraftUpdate,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::api::knowledge::Document> {
    match cms_core::api::knowledge::update_draft(&id, &input).await {
        Ok(article) => match cms_core::api::knowledge::project_article(article) {
            Ok(data) => {
                app.state::<CmsState>()
                    .invalidate_view(cms_core::consumer::Channel::Knowledge)
                    .await;
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
