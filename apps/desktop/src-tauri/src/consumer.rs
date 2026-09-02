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
    memos: CommandResponse<consumers::view::ChannelView>,
    moment: CommandResponse<consumers::view::ChannelView>,
    knowledge: CommandResponse<consumers::view::ChannelView>,
}

#[tauri::command]
pub(crate) async fn initialize_views(app: tauri::AppHandle) -> InitialViews {
    let state = app.state::<CmsState>();
    let (memos, moment, knowledge) = tokio::join!(
        state.channel(ChannelRequest::initial(consumers::view::Channel::Memos,)),
        state.channel(ChannelRequest::initial(consumers::view::Channel::Moment,)),
        state.channel(ChannelRequest::initial(consumers::view::Channel::Knowledge,)),
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
) -> CommandResponse<consumers::view::ChannelView> {
    let paginated = query.cursor.is_some();
    let started = Instant::now();
    let channel = match consumers::view::Channel::try_from(query.channel.as_str()) {
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
            filters: consumers::api::memos::ListFilters {
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
                consumers::view::ChannelView::Memos {
                    memos, next_cursor, ..
                } => (memos.len(), next_cursor.is_some()),
                consumers::view::ChannelView::Moment {
                    photos,
                    next_cursor,
                    ..
                } => (photos.len(), next_cursor.is_some()),
                consumers::view::ChannelView::Knowledge {
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
pub(crate) async fn read_memo_tags() -> CommandResponse<Vec<consumers::api::memos::TagCount>> {
    match consumers::api::memos::tags().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn read_moment_tags() -> CommandResponse<Vec<String>> {
    match consumers::api::moment::tags().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn create_memo(
    content: String,
    visibility: consumers::api::memos::Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::memos::MemoView> {
    match consumers::api::memos::create(&content, visibility).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(consumers::view::Channel::Memos)
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
    visibility: consumers::api::memos::Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::memos::MemoView> {
    match consumers::api::memos::import_x(&url, visibility).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(consumers::view::Channel::Memos)
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
    input: consumers::api::memos::Update,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::memos::MemoView> {
    match consumers::api::memos::update(&id, &input).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(consumers::view::Channel::Memos)
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
    match consumers::api::memos::delete(&id).await {
        Ok(()) => {
            app.state::<CmsState>()
                .invalidate_view(consumers::view::Channel::Memos)
                .await;
            CommandResponse::Ready { data: id }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn publish_telegram(
    id: String,
    app: tauri::AppHandle,
) -> CommandResponse<social::PublishedPost> {
    let memo = match consumers::api::memos::read(&id).await {
        Ok(memo) => memo.memo,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let memo = social::MemoPublication {
        id: memo.id,
        content: memo.content,
        visibility: match memo.visibility {
            consumers::api::memos::Visibility::Public => social::PublicationVisibility::Public,
            consumers::api::memos::Visibility::Private => social::PublicationVisibility::Private,
        },
    };
    let authorization = app.state::<crate::telegram::TelegramAuthorizationState>();
    if let Err(message) = authorization.begin_operation().await {
        return CommandResponse::Failed { message };
    }
    let session_path = match app.path().app_data_dir() {
        Ok(path) => path.join("telegram.session"),
        Err(error) => {
            authorization.finish_operation().await;
            return CommandResponse::Failed {
                message: format!("could not resolve Telegram session storage: {error}"),
            };
        }
    };
    let response = match social::publish_telegram(&memo, &session_path).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    };
    authorization.finish_operation().await;
    response
}

#[tauri::command]
pub(crate) async fn publish_x(
    id: String,
    app: tauri::AppHandle,
) -> CommandResponse<social::PublishedPost> {
    let state = app.state::<crate::configuration::PublicationState>();
    let _operation = state.x_operation.lock().await;
    let memo = match consumers::api::memos::read(&id).await {
        Ok(memo) => memo.memo,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let memo = social::MemoPublication {
        id: memo.id,
        content: memo.content,
        visibility: match memo.visibility {
            consumers::api::memos::Visibility::Public => social::PublicationVisibility::Public,
            consumers::api::memos::Visibility::Private => social::PublicationVisibility::Private,
        },
    };
    match social::publish_x(&memo).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn create_photo(
    input: consumers::api::moment::Upload,
    source: Vec<u8>,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::moment::Photo> {
    let state = app.state::<CmsState>();
    let repository = match state.repository().await {
        Ok(repository) => repository,
        Err(message) => return CommandResponse::Failed { message },
    };
    match consumers::api::moment::upload(repository.store(), input, source).await {
        Ok(data) => {
            state
                .invalidate_view(consumers::view::Channel::Moment)
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
    input: consumers::api::moment::Update,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::moment::Photo> {
    match consumers::api::moment::update(&id, &input).await {
        Ok(data) => {
            app.state::<CmsState>()
                .invalidate_view(consumers::view::Channel::Moment)
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
    match consumers::api::moment::delete(&id).await {
        Ok(()) => {
            let state = app.state::<CmsState>();
            state
                .invalidate_view(consumers::view::Channel::Moment)
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
    input: consumers::api::knowledge::Draft,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::knowledge::Document> {
    let input = consumers::api::knowledge::Create::Draft(input);
    match consumers::api::knowledge::create(&input).await {
        Ok(article) => match consumers::api::knowledge::project_article(article).await {
            Ok(data) => {
                app.state::<CmsState>()
                    .invalidate_view(consumers::view::Channel::Knowledge)
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
    input: consumers::api::knowledge::DraftUpdate,
    app: tauri::AppHandle,
) -> CommandResponse<consumers::api::knowledge::Document> {
    match consumers::api::knowledge::update_draft(&id, &input).await {
        Ok(article) => match consumers::api::knowledge::project_article(article).await {
            Ok(data) => {
                app.state::<CmsState>()
                    .invalidate_view(consumers::view::Channel::Knowledge)
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
