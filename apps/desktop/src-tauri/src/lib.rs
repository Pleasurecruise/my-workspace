use std::error::Error;

use cms::CmsState;
use tauri::http::{Response, StatusCode, header};
use tauri::{Emitter, Manager};
use tracing_subscriber::EnvFilter;

mod cms;
mod configuration;
mod consumer;
mod dashboard;
mod gaming;
mod island;
mod ledger;
mod music;
mod notifications;
mod status;
mod storage;
mod telegram;
mod telemetry;
mod terminal;
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
    if let Err(error) = init_logging() {
        panic!("failed to initialize logging: {error}");
    }
    tracing::info!("starting desktop application");

    let result = tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("vesper-asset", |context, request, responder| {
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
        })
        .register_asynchronous_uri_scheme_protocol(
            "vesper-music-cover",
            |context, request, responder| {
                if context.webview_label() != "main" || request.method() != tauri::http::Method::GET
                {
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
        .manage(configuration::AppLockState::default())
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
        .on_page_load(|window, payload| {
            if window.label() != "main"
                || !matches!(payload.event(), tauri::webview::PageLoadEvent::Started)
            {
                return;
            }
            if let Some(runtime) = window.app_handle().try_state::<terminal::Runtime>() {
                runtime.suspend();
            }
        })
        .on_window_event(|window, event| {
            if window.label() != "main" || !matches!(event, tauri::WindowEvent::Destroyed) {
                return;
            }
            if let Some(runtime) = window.app_handle().try_state::<terminal::Runtime>() {
                runtime.suspend();
            }
        })
        .setup(|app| {
            app.manage(terminal::Runtime::default());
            terminal::start_monitoring(app.handle().clone());
            app.manage(games::Runtime::new(
                app.path()
                    .app_local_data_dir()?
                    .join(vesper_database::FILE_NAME),
            ));
            app.manage(todo_core::Store::shared()?);
            app.manage(::ledger::Store::new(vesper_database::shared_path()?));
            let notifications_path = app
                .path()
                .app_local_data_dir()?
                .join(vesper_database::FILE_NAME);
            app.manage(notifications::NotificationState::new(notifications_path));
            island::sync(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut previous_date = None;
                loop {
                    match todo_core::current_date() {
                        Ok(date) => {
                            if let Err(error) = todo::roll_over(&handle, &date).await {
                                tracing::error!(%error, "failed to roll over unfinished Todos");
                            }
                            if previous_date.as_ref() != Some(&date) {
                                if let Err(error) = handle.emit("planner-date-changed", &date) {
                                    tracing::warn!(%error, "failed to notify the Planner date");
                                } else {
                                    previous_date = Some(date);
                                }
                            }
                        }
                        Err(error) => tracing::error!(%error, "failed to resolve the Planner date"),
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            terminal::set_terminal_active,
            terminal::read_ssh_devices,
            terminal::connect_terminal,
            terminal::write_terminal,
            terminal::record_terminal_activity,
            terminal::resize_terminal,
            terminal::acknowledge_terminal,
            terminal::disconnect_terminal,
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
            consumer::read_photo_metadata,
            consumer::create_photo,
            consumer::update_photo,
            consumer::delete_photo,
            consumer::create_knowledge,
            consumer::update_knowledge,
            consumer::read_knowledge,
            consumer::prefetch_knowledge,
            consumer::markdown_matches,
            consumer::markdown_spans,
            consumer::preview_knowledge,
            updater::check_for_update,
            updater::install_update,
            dashboard::refresh_dashboard,
            dashboard::refresh_island,
            island::island_available,
            island::set_island_expanded,
            dashboard::set_dashboard_active,
            status::read_service_catalog,
            storage::open_storage_settings,
            widgets::read_layout,
            widgets::reset_layout,
            widgets::save_layout,
            ledger::read_expenses,
            ledger::create_expense,
            ledger::update_expense,
            ledger::delete_expense,
            todo::read_todos,
            todo::read_planner_date,
            todo::read_planner_days,
            todo::read_check_ins,
            todo::set_check_in,
            todo::add_todo,
            todo::update_todo,
            todo::set_todo_completed,
            todo::set_todo_rollover,
            todo::delete_todo,
            todo::reorder_todos,
            configuration::read_configuration,
            configuration::save_ugos_configuration,
            configuration::save_r2_configuration,
            configuration::save_api_configuration,
            configuration::save_telegram,
            configuration::connect_x,
            telegram::read_auth,
            telegram::begin_auth,
            telegram::submit_code,
            telegram::submit_password,
            telegram::cancel_auth,
            configuration::save_ntfy_configuration,
            configuration::save_notion_calendar,
            configuration::save_codex_resets,
            notifications::set_notifications_active,
            notifications::read_notifications,
            notifications::mark_notification_read,
            configuration::save_app_lock,
            configuration::remove_app_lock,
            configuration::unlock_app,
            configuration::lock_app,
            configuration::read_app_lock,
            gaming::read_game_connections,
            gaming::select_game_account,
            gaming::remove_game_account,
            gaming::begin_game_login,
            gaming::poll_game_login,
            gaming::cancel_game_login,
            gaming::save_steam_connection,
            gaming::read_steam_settings,
            gaming::read_game_notes,
            gaming::verify_game,
            gaming::read_steam_games,
            gaming::read_gacha_archive,
            gaming::sync_gacha_archive,
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
        .build(tauri::generate_context!());
    let app =
        result.unwrap_or_else(|error| panic!("error while building tauri application: {error}"));
    app.run(|app, event| {
        if !matches!(event, tauri::RunEvent::Exit) {
            return;
        }
        if let Some(runtime) = app.try_state::<terminal::Runtime>() {
            runtime.suspend();
        }
    });
}

fn init_logging() -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value)?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(error) => return Err(Box::new(error)),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()?;

    Ok(())
}
