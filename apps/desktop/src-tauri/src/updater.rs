use crate::CommandResponse;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
#[cfg(not(target_os = "macos"))]
use tauri::menu::HELP_SUBMENU_ID;
use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const CHECK_MENU_ID: &str = "check-for-updates";
#[cfg(debug_assertions)]
const RELOAD_MENU_ID: &str = "reload-webview";
#[cfg(debug_assertions)]
const TOGGLE_DEVTOOLS_MENU_ID: &str = "toggle-developer-tools";
const CHECK_FOR_UPDATES_EVENT: &str = "check-for-updates-requested";
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
pub(crate) struct UpdateState {
    operation: OperationGate,
}

#[derive(Default)]
struct OperationGate(AtomicBool);

impl OperationGate {
    fn enter(&self) -> Option<OperationLease<'_>> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| OperationLease { gate: self })
    }
}

struct OperationLease<'a> {
    gate: &'a OperationGate,
}

impl Drop for OperationLease<'_> {
    fn drop(&mut self) {
        self.gate.0.store(false, Ordering::Release);
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    current_version: String,
    version: String,
    notes: Option<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum UpdateProgress {
    Downloading {
        downloaded: usize,
        total: Option<u64>,
    },
    Downloaded,
}

pub(crate) fn menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    let check_for_updates =
        MenuItem::with_id(app, CHECK_MENU_ID, "Check for Updates…", true, None::<&str>)?;

    #[cfg(target_os = "macos")]
    if let Some(MenuItemKind::Submenu(application_menu)) = menu.items()?.into_iter().next() {
        application_menu.insert(&check_for_updates, 1)?;
    }

    #[cfg(all(target_os = "macos", debug_assertions))]
    for item in menu.items()? {
        let MenuItemKind::Submenu(submenu) = item else {
            continue;
        };
        if submenu.text()? != "View" {
            continue;
        }
        let reload = MenuItem::with_id(app, RELOAD_MENU_ID, "Reload", true, Some("CmdOrCtrl+R"))?;
        let toggle_devtools = MenuItem::with_id(
            app,
            TOGGLE_DEVTOOLS_MENU_ID,
            "Toggle Developer Tools",
            true,
            Some("CmdOrCtrl+Alt+I"),
        )?;
        submenu.prepend(&toggle_devtools)?;
        submenu.prepend(&reload)?;
        break;
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(MenuItemKind::Submenu(help_menu)) = menu.get(HELP_SUBMENU_ID) {
        help_menu.prepend(&check_for_updates)?;
    }

    Ok(menu)
}

pub(crate) fn handle_menu_event(app: &tauri::AppHandle, event: &MenuEvent) {
    #[cfg(debug_assertions)]
    if event.id() == RELOAD_MENU_ID {
        if let Some(webview) = app.get_webview_window("main")
            && let Err(error) = webview.reload()
        {
            tracing::warn!(%error, "failed to reload the main webview");
        }
        return;
    }

    #[cfg(debug_assertions)]
    if event.id() == TOGGLE_DEVTOOLS_MENU_ID {
        if let Some(webview) = app.get_webview_window("main") {
            if webview.is_devtools_open() {
                webview.close_devtools();
            } else {
                webview.open_devtools();
            }
        }
        return;
    }

    if event.id() != CHECK_MENU_ID {
        return;
    }
    if let Err(error) = app.emit(CHECK_FOR_UPDATES_EVENT, ()) {
        tracing::warn!(%error, "failed to request a manual application update check");
    }
}

#[tauri::command]
pub(crate) async fn check_for_update(app: tauri::AppHandle) -> CommandResponse<Option<UpdateInfo>> {
    let state = app.state::<UpdateState>();
    let Some(_checking) = state.operation.enter() else {
        return CommandResponse::Failed {
            message: "Another application update operation is already running.".to_owned(),
        };
    };
    let updater = match app.updater_builder().timeout(CHECK_TIMEOUT).build() {
        Ok(updater) => updater,
        Err(error) => {
            return CommandResponse::Failed {
                message: format!("Could not initialize the application updater: {error}"),
            };
        }
    };
    match updater.check().await {
        Ok(Some(update))
            if update.current_version.trim_start_matches('v')
                == update.version.trim_start_matches('v') =>
        {
            tracing::warn!(
                current_version = %update.current_version,
                available_version = %update.version,
                "updater returned the installed version as an available update"
            );
            CommandResponse::Ready { data: None }
        }
        Ok(Some(update)) => CommandResponse::Ready {
            data: Some(UpdateInfo {
                current_version: update.current_version,
                version: update.version,
                notes: update.body,
            }),
        },
        Ok(None) => CommandResponse::Ready { data: None },
        Err(error) => {
            let details = error.to_string();
            let message = if details.contains("error sending request") {
                format!(
                    "Could not reach GitHub to check for updates. Check the network and system proxy settings, then try again. ({details})"
                )
            } else {
                format!("Could not check for application updates: {details}")
            };
            CommandResponse::Failed { message }
        }
    }
}

#[tauri::command]
pub(crate) async fn install_update(
    version: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<UpdateState>();
    let Some(_installing) = state.operation.enter() else {
        return CommandResponse::Failed {
            message: "Another application update operation is already running.".to_owned(),
        };
    };

    let result = async {
        let update = app
            .updater_builder()
            .timeout(CHECK_TIMEOUT)
            .build()
            .map_err(|error| format!("Could not initialize the application updater: {error}"))?
            .check()
            .await
            .map_err(|error| format!("Could not check the application update: {error}"))?
            .ok_or_else(|| "The application update is no longer available.".to_owned())?;
        if update.current_version.trim_start_matches('v')
            == update.version.trim_start_matches('v')
        {
            return Err("The application is already up to date.".to_owned());
        }
        if update.version != version {
            return Err(format!(
                "The available application update changed from {version} to {}. Please review it before installing.",
                update.version
            ));
        }

        let progress_app = app.clone();
        let downloaded_app = app.clone();
        let mut downloaded = 0_usize;
        tokio::time::timeout(
            INSTALL_TIMEOUT,
            update.download_and_install(
                move |chunk, total| {
                    downloaded += chunk;
                    if let Err(error) = progress_app.emit(
                        "updater-progress",
                        UpdateProgress::Downloading { downloaded, total },
                    ) {
                        tracing::warn!(%error, "failed to emit application update progress");
                    }
                },
                move || {
                    if let Err(error) =
                        downloaded_app.emit("updater-progress", UpdateProgress::Downloaded)
                    {
                        tracing::warn!(%error, "failed to emit application update completion");
                    }
                },
            ),
        )
            .await
            .map_err(|_| {
                format!(
                    "The application update timed out after {} minutes.",
                    INSTALL_TIMEOUT.as_secs() / 60
                )
            })?
            .map_err(|error| format!("Could not install the application update: {error}"))?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(message) = result {
        return CommandResponse::Failed { message };
    }
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_rejects_overlap() {
        let gate = OperationGate::default();
        let lease = gate.enter().expect("first operation should start");

        assert!(gate.enter().is_none());

        drop(lease);
    }

    #[test]
    fn gate_reopens() {
        let gate = OperationGate::default();

        let lease = gate.enter().expect("first operation should start");
        drop(lease);

        assert!(gate.enter().is_some());
    }
}
