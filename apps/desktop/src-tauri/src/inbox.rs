use crate::CommandResponse;
use inbox::{Inbox, Listener, Notification};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

const FRESH_SECONDS: i64 = 60;

#[tauri::command]
pub(crate) async fn read_notifications(
    app: tauri::AppHandle,
) -> CommandResponse<Vec<Notification>> {
    app.state::<Inbox>().read().await.into()
}

#[tauri::command]
pub(crate) async fn mark_notification_read(
    id: String,
    app: tauri::AppHandle,
) -> CommandResponse<Vec<Notification>> {
    let result = app.state::<Inbox>().mark_read(&id).await;
    if let Ok(notifications) = &result {
        emit(&app, notifications.clone());
    }
    result.into()
}

#[tauri::command]
pub(crate) async fn set_notifications_active(
    active: bool,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    let listener = listener(&app);
    app.state::<Inbox>()
        .subscribe(Some(active), listener)
        .await
        .into()
}

pub(crate) async fn restart(app: &tauri::AppHandle) -> Result<(), String> {
    app.state::<Inbox>().subscribe(None, listener(app)).await
}

fn listener(app: &tauri::AppHandle) -> Listener {
    let app = app.clone();
    Arc::new(move |notifications| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        if let Some(latest) = notifications.first()
            && now.is_some_and(|now| latest.timestamp >= now - FRESH_SECONDS)
        {
            let title = latest.title.as_deref().unwrap_or(&latest.source);
            let shown = app
                .notification()
                .builder()
                .title(title)
                .body(&latest.message)
                .show();
            if let Err(error) = shown {
                tracing::debug!(%error, "operating-system notification was not shown");
            }
        }
        emit(&app, notifications);
    })
}

fn emit(app: &tauri::AppHandle, notifications: Vec<Notification>) {
    if let Err(error) = app.emit("notifications-updated", notifications) {
        tracing::warn!(%error, "could not emit notification update");
    }
}
