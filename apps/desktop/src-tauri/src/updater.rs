use crate::CommandResponse;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Default)]
pub(crate) struct UpdateState {
    installing: tokio::sync::Mutex<bool>,
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

#[tauri::command]
pub(crate) async fn check_for_update(app: tauri::AppHandle) -> CommandResponse<Option<UpdateInfo>> {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => {
            return CommandResponse::Failed {
                message: format!("Could not initialize the application updater: {error}"),
            };
        }
    };
    match updater.check().await {
        Ok(Some(update)) => CommandResponse::Ready {
            data: Some(UpdateInfo {
                current_version: update.current_version,
                version: update.version,
                notes: update.body,
            }),
        },
        Ok(None) => CommandResponse::Ready { data: None },
        Err(error) => CommandResponse::Failed {
            message: format!("Could not check for application updates: {error}"),
        },
    }
}

#[tauri::command]
pub(crate) async fn install_update(
    version: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    {
        let state = app.state::<UpdateState>();
        let mut installing = state.installing.lock().await;
        if *installing {
            return CommandResponse::Failed {
                message: "An application update is already being installed.".to_owned(),
            };
        }
        *installing = true;
    }

    let result = async {
        let update = app
            .updater()
            .map_err(|error| format!("Could not initialize the application updater: {error}"))?
            .check()
            .await
            .map_err(|error| format!("Could not check the application update: {error}"))?
            .ok_or_else(|| "The application update is no longer available.".to_owned())?;
        if update.version != version {
            return Err(format!(
                "The available application update changed from {version} to {}. Please review it before installing.",
                update.version
            ));
        }

        let progress_app = app.clone();
        let downloaded_app = app.clone();
        let mut downloaded = 0_usize;
        update
            .download_and_install(
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
            )
            .await
            .map_err(|error| format!("Could not install the application update: {error}"))?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(message) = result {
        let state = app.state::<UpdateState>();
        *state.installing.lock().await = false;
        return CommandResponse::Failed { message };
    }
    app.restart();
}
