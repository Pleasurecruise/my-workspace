use cms::CmsState;
use tauri::{Emitter, Manager};

mod cms;
mod configuration;
mod consumer;
mod dashboard;
mod github;
mod notifications;
mod stocks;
mod todo;
mod updater;
mod weather;
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
        .manage(CmsState::default())
        .manage(dashboard::DashboardRuntime::default())
        .manage(updater::UpdateState::default())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .menu(updater::menu)
        .on_menu_event(|app, event| updater::handle_menu_event(app, &event))
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
            consumer::initialize_views,
            consumer::read_channel,
            consumer::read_memo_tags,
            consumer::read_moment_tags,
            consumer::read_asset,
            consumer::create_memo,
            consumer::import_x_memo,
            consumer::update_memo,
            consumer::delete_memo,
            consumer::create_photo,
            consumer::update_photo,
            consumer::delete_photo,
            consumer::create_knowledge,
            consumer::update_knowledge,
            updater::check_for_update,
            updater::install_update,
            dashboard::refresh_dashboard,
            dashboard::set_dashboard_active,
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
            configuration::save_ntfy_configuration,
            notifications::read_notifications,
            notifications::mark_notification_read,
            configuration::save_app_lock,
            configuration::remove_app_lock,
            configuration::unlock_app
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        panic!("error while running tauri application: {error}");
    }
}
