use cms::CmsState;
use tauri::http::{Response, StatusCode, header};
use tauri::{Emitter, Manager};

mod cms;
mod configuration;
mod consumer;
mod dashboard;
mod music;
mod notifications;
mod status;
mod storage;
mod telegram;
mod telemetry;
mod todo;
mod updater;
mod widgets;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum CommandResponse<T> {
    Ready { data: T },
    Failed { message: String },
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    if let Err(error) = vesper_credentials::load_dev_environment() {
        panic!("failed to load development credentials: {error}");
    }
    if let Err(error) = my_workspace_logger::init() {
        panic!("failed to initialize logging: {error}");
    }
    my_workspace_logger::info!("starting desktop application");

    let result = tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(
            "vesper-asset",
            |context, request, responder| {
                if context.webview_label() != "main" {
                    responder.respond(
                        Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Vec::new())
                            .expect("static asset response should build"),
                    );
                    return;
                }
                if request.method() != tauri::http::Method::GET {
                    responder.respond(
                        Response::builder()
                            .status(StatusCode::METHOD_NOT_ALLOWED)
                            .body(Vec::new())
                            .expect("static asset response should build"),
                    );
                    return;
                }
                let app = context.app_handle().clone();
                let key = match percent_encoding::percent_decode_str(
                    request.uri().path().trim_start_matches('/'),
                )
                .decode_utf8()
                {
                    Ok(key) => key.into_owned(),
                    Err(_) => {
                        responder.respond(
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Vec::new())
                                .expect("static asset response should build"),
                        );
                        return;
                    }
                };
                let content_type = match key.rsplit_once('.').map(|(_, extension)| extension) {
                    Some(extension) if extension.eq_ignore_ascii_case("png") => "image/png",
                    Some(extension)
                        if extension.eq_ignore_ascii_case("jpg")
                            || extension.eq_ignore_ascii_case("jpeg") =>
                    {
                        "image/jpeg"
                    }
                    Some(extension) if extension.eq_ignore_ascii_case("webp") => "image/webp",
                    Some(extension) if extension.eq_ignore_ascii_case("avif") => "image/avif",
                    _ => {
                        responder.respond(
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Vec::new())
                                .expect("static asset response should build"),
                        );
                        return;
                    }
                };
                tauri::async_runtime::spawn(async move {
                    let response = match app.state::<CmsState>().asset(&key).await {
                        Ok(data) => Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, content_type)
                            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
                            .header(header::CACHE_CONTROL, "no-store")
                            .body(data.as_ref().clone())
                            .expect("static asset response should build"),
                        Err(error) => {
                            tracing::warn!(%error, %key, "could not serve a Moment image");
                            Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Vec::new())
                                .expect("static asset response should build")
                        }
                    };
                    responder.respond(response);
                });
            },
        )
        .register_asynchronous_uri_scheme_protocol(
            "vesper-music-cover",
            |context, request, responder| {
                if context.webview_label() != "main" || request.method() != tauri::http::Method::GET {
                    responder.respond(
                        Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Vec::new())
                            .expect("music cover response should build"),
                    );
                    return;
                }
                let key = percent_encoding::percent_decode_str(
                    request.uri().path().trim_start_matches('/'),
                )
                .decode_utf8()
                .map(|key| key.into_owned());
                let app = context.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let response = match key {
                        Ok(key) => match app.state::<music::MusicState>().cover(&key).await {
                            Ok(cover) => Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, cover.content_type)
                                .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
                                .header(header::CACHE_CONTROL, "private, max-age=86400")
                                .body(cover.bytes)
                                .expect("music cover response should build"),
                            Err(error) => {
                                tracing::warn!(%error, %key, "could not serve a music album cover");
                                Response::builder()
                                    .status(StatusCode::NOT_FOUND)
                                    .body(Vec::new())
                                    .expect("music cover response should build")
                            }
                        },
                        Err(_) => Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Vec::new())
                            .expect("music cover response should build"),
                    };
                    responder.respond(response);
                });
            },
        )
        .manage(CmsState::default())
        .manage(configuration::PublicationState::default())
        .manage(telegram::TelegramAuthorizationState::default())
        .manage(music::MusicState::default())
        .manage(dashboard::DashboardRuntime::default())
        .manage(updater::UpdateState::default())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .menu(updater::menu)
        .on_menu_event(|app, event| updater::handle_menu_event(app, &event))
        .setup(|app| {
            let todo_path = app.path().app_data_dir()?.join("todos.json");
            app.manage(todo_core::Store::new(todo_path));
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
                    let delay = match todo_core::next_rollover_delay() {
                        Ok(delay) => delay,
                        Err(error) => {
                            tracing::error!(%error, "failed to schedule Todo rollover");
                            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                            continue;
                        }
                    };
                    tokio::time::sleep(delay).await;
                    let date = match todo_core::current_date() {
                        Ok(date) => date,
                        Err(error) => {
                            tracing::error!(%error, "failed to resolve the date for Todo rollover");
                            continue;
                        }
                    };
                    match handle
                        .state::<todo_core::Store>()
                        .sync_schedule(&date)
                        .await
                    {
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
            consumer::initialize_views,
            consumer::read_channel,
            consumer::read_memo_tags,
            consumer::read_moment_tags,
            consumer::create_memo,
            consumer::import_x_memo,
            consumer::update_memo,
            consumer::delete_memo,
            consumer::publish_telegram,
            consumer::publish_x,
            consumer::create_photo,
            consumer::update_photo,
            consumer::delete_photo,
            consumer::create_knowledge,
            consumer::update_knowledge,
            updater::check_for_update,
            updater::install_update,
            dashboard::refresh_dashboard,
            dashboard::set_dashboard_active,
            status::read_service_status_catalog,
            storage::read_storage,
            widgets::read_layout,
            widgets::reset_layout,
            widgets::save_layout,
            todo::read_todos,
            todo::add_todo,
            todo::set_todo_completed,
            todo::delete_todo,
            configuration::read_configuration,
            configuration::save_ugos_configuration,
            configuration::save_r2_configuration,
            configuration::save_api_configuration,
            configuration::read_publication,
            configuration::save_telegram,
            configuration::connect_x,
            telegram::read_auth,
            telegram::begin_auth,
            telegram::submit_code,
            telegram::submit_password,
            telegram::cancel_auth,
            configuration::save_ntfy_configuration,
            notifications::read_notifications,
            notifications::mark_notification_read,
            configuration::save_app_lock,
            configuration::remove_app_lock,
            configuration::unlock_app,
            music::connect_spotify,
            music::begin_qq_music_login,
            music::poll_qq_music_login,
            music::cancel_qq_music_login,
            music::read_music_tracks,
            music::read_music_playback,
            music::play_music_track,
            music::resume_music,
            music::pause_music,
            music::seek_music,
            music::set_music_playback_order,
            music::read_music_lyrics
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        panic!("error while running tauri application: {error}");
    }
}
