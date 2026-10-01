use std::error::Error;

use tauri::{Emitter, Manager};
use tracing_subscriber::EnvFilter;

mod app_lock;
mod chat;
mod content;
mod dashboard;
mod games;
mod inbox;
mod island;
mod ledger;
mod music;
mod protocol;
mod settings;
mod social;
mod storage;
mod telemetry;
mod terminal;
mod todo;
mod updater;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum CommandResponse<T> {
    Ready { data: T },
    Failed { message: String },
}

impl<T, E: std::fmt::Display> From<Result<T, E>> for CommandResponse<T> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(data) => Self::Ready { data },
            Err(error) => Self::Failed {
                message: error.to_string(),
            },
        }
    }
}

/// Command-body error: accepts any displayable error through `?`. It deliberately does not
/// implement `Display`, which keeps both conversions below coherent.
pub(crate) struct CommandError(String);

impl<E: std::fmt::Display> From<E> for CommandError {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

impl<T> From<Result<T, CommandError>> for CommandResponse<T> {
    fn from(result: Result<T, CommandError>) -> Self {
        match result {
            Ok(data) => Self::Ready { data },
            Err(CommandError(message)) => Self::Failed { message },
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    if let Err(error) = vault::load_dev_environment() {
        panic!("failed to load development credentials: {error}");
    }
    if let Err(error) = init_logging() {
        panic!("failed to initialize logging: {error}");
    }
    tracing::info!("starting desktop application");

    let result = tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("vesper-asset", protocol::asset)
        .register_asynchronous_uri_scheme_protocol("vesper-music-cover", protocol::music_cover)
        .manage(island::Visibility::default())
        .manage(chat::Runtime::default())
        .manage(content::Content::default())
        .manage(app_lock::AppLock::default())
        .manage(social::Social::default())
        .manage(music::Music::default())
        .manage(dashboard::Runtime::default())
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
            window
                .app_handle()
                .state::<chat::Runtime>()
                .suspend(window.app_handle());
        })
        .on_window_event(|window, event| {
            if window.label() != "main" || !matches!(event, tauri::WindowEvent::Destroyed) {
                return;
            }
            if let Some(runtime) = window.app_handle().try_state::<terminal::Runtime>() {
                runtime.suspend();
            }
            window
                .app_handle()
                .state::<chat::Runtime>()
                .suspend(window.app_handle());
        })
        .setup(|app| {
            app.manage(terminal::Runtime::default());
            terminal::start_monitoring(app.handle().clone());
            app.manage(::games::Runtime::new(database::path()?));
            app.manage(::todo::Store::shared()?);
            app.manage(::ledger::Store::new(database::path()?));
            app.manage(::inbox::Inbox::new(database::path()?));
            island::sync(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut previous_date = None;
                loop {
                    match ::todo::current_date() {
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
            app_lock::save_app_lock,
            app_lock::remove_app_lock,
            app_lock::unlock_app,
            app_lock::lock_app,
            app_lock::read_app_lock,
            chat::read_chat,
            chat::connect_chat,
            chat::send_chat,
            chat::control_chat,
            content::initialize_views,
            content::read_channel,
            content::memos::read_memo_tags,
            content::memos::create_memo,
            content::memos::import_x_memo,
            content::memos::update_memo,
            content::memos::delete_memo,
            content::moment::read_moment_tags,
            content::moment::read_photo_metadata,
            content::moment::create_photo,
            content::moment::update_photo,
            content::moment::delete_photo,
            content::knowledge::create_knowledge,
            content::knowledge::update_knowledge,
            content::knowledge::read_knowledge,
            content::knowledge::prefetch_knowledge,
            content::knowledge::markdown_matches,
            content::knowledge::markdown_spans,
            content::knowledge::preview_knowledge,
            dashboard::refresh_dashboard,
            dashboard::refresh_island,
            dashboard::set_dashboard_active,
            dashboard::read_service_catalog,
            dashboard::read_layout,
            dashboard::reset_layout,
            dashboard::save_layout,
            games::read_game_connections,
            games::select_game_account,
            games::remove_game_account,
            games::begin_game_login,
            games::poll_game_login,
            games::cancel_game_login,
            games::save_steam_connection,
            games::read_steam_settings,
            games::read_game_notes,
            games::verify_game,
            games::read_steam_games,
            games::read_gacha_archive,
            games::sync_gacha_archive,
            inbox::set_notifications_active,
            inbox::read_notifications,
            inbox::mark_notification_read,
            island::island_available,
            island::read_island_visible,
            island::set_island_visible,
            island::set_island_expanded,
            ledger::read_expenses,
            ledger::create_expense,
            ledger::update_expense,
            ledger::delete_expense,
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
            music::read_music_lyrics,
            settings::read_configuration,
            settings::save_ugos_configuration,
            settings::save_r2_configuration,
            settings::save_api_configuration,
            settings::save_ntfy_configuration,
            settings::save_notion_calendar,
            settings::save_codex_resets,
            social::publish_telegram,
            social::publish_x,
            social::save_telegram,
            social::connect_x,
            social::read_auth,
            social::begin_auth,
            social::submit_code,
            social::submit_password,
            social::cancel_auth,
            storage::open_storage_settings,
            terminal::set_terminal_active,
            terminal::read_ssh_devices,
            terminal::connect_terminal,
            terminal::write_terminal,
            terminal::record_terminal_activity,
            terminal::resize_terminal,
            terminal::acknowledge_terminal,
            terminal::disconnect_terminal,
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
            updater::check_for_update,
            updater::install_update
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
        app.state::<chat::Runtime>().suspend(app);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_errors_become_failed_responses() {
        let result: Result<u8, CommandError> = (|| {
            let value: u8 = "300".parse()?;
            Ok(value)
        })();
        let CommandResponse::Failed { message } = CommandResponse::from(result) else {
            panic!("expected a failed response");
        };
        assert!(message.contains("too large"));
        let ready = CommandResponse::from(Ok::<_, CommandError>(7));
        assert!(matches!(ready, CommandResponse::Ready { data: 7 }));
        let displayed = CommandResponse::<u8>::from(Err::<u8, _>("offline"));
        assert!(matches!(displayed, CommandResponse::Failed { message } if message == "offline"));
    }
}
