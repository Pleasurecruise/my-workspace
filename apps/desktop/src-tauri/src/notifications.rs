use crate::CommandResponse;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

const NOTIFICATION_LIMIT: usize = 200;
const NTFY_SUBSCRIPTION_URL: &str = "https://ntfy.you-find.me/mail-summary/sse";
const NTFY_TOPIC: &str = "mail-summary";
const SSE_LINE_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notification {
    id: String,
    topic: String,
    source: String,
    title: Option<String>,
    message: String,
    timestamp: i64,
    tags: Vec<String>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct NotificationStore {
    last_id: Option<String>,
    notifications: Vec<Notification>,
}

#[derive(Deserialize)]
struct NtfyMessage {
    id: String,
    time: i64,
    event: String,
    topic: String,
    title: Option<String>,
    message: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct NotificationEnvelope {
    source: String,
    title: Option<String>,
    body: String,
}

pub(crate) struct NotificationState {
    path: PathBuf,
    store: RwLock<NotificationStore>,
    subscription: Mutex<Option<JoinHandle<()>>>,
}

impl NotificationState {
    pub(crate) fn new(path: PathBuf) -> Result<Self, String> {
        let store = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("could not parse {}: {error}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                NotificationStore::default()
            }
            Err(error) => {
                return Err(format!("could not read {}: {error}", path.display()));
            }
        };
        Ok(Self {
            path,
            store: RwLock::new(store),
            subscription: Mutex::new(None),
        })
    }

    async fn accept(&self, message: NtfyMessage) -> Result<Option<Vec<Notification>>, String> {
        if message.event != "message" {
            return Ok(None);
        }
        if message.topic != NTFY_TOPIC || message.time < 0 || message.time > 8_640_000_000 {
            return Err("ntfy returned invalid notification metadata".to_owned());
        }
        let Some(body) = message.message.filter(|body| !body.trim().is_empty()) else {
            return Ok(None);
        };
        let envelope = serde_json::from_str::<NotificationEnvelope>(&body).ok();
        let source = envelope
            .as_ref()
            .map(|envelope| envelope.source.trim())
            .filter(|source| !source.is_empty())
            .unwrap_or(&message.topic)
            .to_owned();
        let title = envelope
            .as_ref()
            .and_then(|envelope| envelope.title.clone())
            .or(message.title);
        let body = envelope
            .map(|envelope| envelope.body)
            .filter(|body| !body.trim().is_empty())
            .unwrap_or(body);
        if source.len() > 200
            || title.as_ref().is_some_and(|title| title.len() > 500)
            || body.len() > 500_000
            || message.tags.len() > 50
            || message.tags.iter().any(|tag| tag.len() > 100)
        {
            return Err("ntfy returned an oversized notification".to_owned());
        }
        let mut store = self.store.write().await;
        if store.notifications.iter().any(|item| item.id == message.id) {
            return Ok(None);
        }
        let mut next = store.clone();
        next.last_id = Some(message.id.clone());
        next.notifications.insert(
            0,
            Notification {
                id: message.id,
                topic: message.topic,
                source,
                title,
                message: body,
                timestamp: message.time,
                tags: message.tags,
            },
        );
        next.notifications.truncate(NOTIFICATION_LIMIT);
        let encoded = serde_json::to_vec(&next).map_err(|error| error.to_string())?;
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| error.to_string())?;
        }
        tokio::fs::write(&self.path, encoded)
            .await
            .map_err(|error| error.to_string())?;
        *store = next;
        Ok(Some(store.notifications.clone()))
    }
}

#[tauri::command]
pub(crate) async fn read_notifications(
    state: tauri::State<'_, NotificationState>,
) -> Result<CommandResponse<Vec<Notification>>, String> {
    Ok(CommandResponse::Ready {
        data: state.store.read().await.notifications.clone(),
    })
}

pub(crate) async fn restart(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<NotificationState>();
    if let Some(subscription) = state.subscription.lock().await.take() {
        subscription.abort();
    }
    let configuration = match vesper_credentials::ntfy().map_err(|error| error.to_string())? {
        vesper_credentials::Stored::Missing => return Ok(()),
        vesper_credentials::Stored::Ready(configuration) => configuration,
    };
    let task = tokio::spawn(run_subscription(app.clone(), configuration.token));
    *state.subscription.lock().await = Some(task);
    Ok(())
}

async fn run_subscription(app: tauri::AppHandle, token: String) {
    let client = match reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(%error, "could not create ntfy client");
            return;
        }
    };
    loop {
        let state = app.state::<NotificationState>();
        let since = match &state.store.read().await.last_id {
            Some(last_id) => last_id.clone(),
            None => "all".to_owned(),
        };
        let response = client
            .get(NTFY_SUBSCRIPTION_URL)
            .bearer_auth(&token)
            .query(&[("since", since)])
            .send()
            .await;
        match response {
            Ok(response) => match response.error_for_status() {
                Ok(response) => consume_stream(&app, response).await,
                Err(error) => {
                    tracing::warn!(%error, "ntfy subscription was rejected");
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                    continue;
                }
            },
            Err(error) => {
                tracing::warn!(%error, "ntfy subscription could not connect");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn consume_stream(app: &tauri::AppHandle, response: reqwest::Response) {
    let state = app.state::<NotificationState>();
    let mut response = response.bytes_stream();
    let mut pending = Vec::new();
    while let Some(chunk) = response.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                tracing::warn!(%error, "ntfy subscription stream ended");
                return;
            }
        };
        pending.extend_from_slice(&chunk);
        if pending.len() > SSE_LINE_LIMIT && !pending.contains(&b'\n') {
            tracing::warn!("ntfy sent an oversized SSE line");
            return;
        }
        while let Some(newline) = pending.iter().position(|byte| *byte == b'\n') {
            if newline > SSE_LINE_LIMIT {
                pending.drain(..=newline);
                tracing::warn!("ntfy sent an oversized SSE line");
                continue;
            }
            let line = pending.drain(..=newline).collect::<Vec<_>>();
            accept_line(app, &state, &line).await;
        }
    }
}

async fn accept_line(app: &tauri::AppHandle, state: &NotificationState, line: &[u8]) {
    let Ok(line) = std::str::from_utf8(line) else {
        tracing::warn!("ntfy sent a non-UTF-8 SSE line");
        return;
    };
    let Some(data) = line.trim_end_matches(['\n', '\r']).strip_prefix("data:") else {
        return;
    };
    let message = match serde_json::from_str::<NtfyMessage>(data.trim_start()) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(%error, "ntfy sent an invalid message payload");
            return;
        }
    };
    match state.accept(message).await {
        Ok(Some(notifications)) => publish_notifications(app, notifications),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "could not persist notification"),
    }
}

fn publish_notifications(app: &tauri::AppHandle, notifications: Vec<Notification>) {
    if let Some(notification) = notifications.first() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        if now.is_some_and(|now| notification.timestamp >= now - 60) {
            let title = notification
                .title
                .as_deref()
                .unwrap_or(&notification.source);
            if let Err(error) = app
                .notification()
                .builder()
                .title(title)
                .body(&notification.message)
                .show()
            {
                tracing::debug!(%error, "operating-system notification was not shown");
            }
        }
    }
    if let Err(error) = app.emit("notifications-updated", notifications) {
        tracing::warn!(%error, "could not emit notification update");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepts_messages_and_deduplicates_ids() {
        let path =
            std::env::temp_dir().join(format!("vesper-notifications-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let state = NotificationState::new(path.clone()).unwrap();
        let message = NtfyMessage {
            id: "message-1".to_owned(),
            time: 1,
            event: "message".to_owned(),
            topic: "mail-summary".to_owned(),
            title: Some("Mail".to_owned()),
            message: Some("Summary".to_owned()),
            tags: vec![],
        };
        assert!(state.accept(message).await.unwrap().is_some());
        let duplicate = NtfyMessage {
            id: "message-1".to_owned(),
            time: 2,
            event: "message".to_owned(),
            topic: "mail-summary".to_owned(),
            title: None,
            message: Some("Duplicate".to_owned()),
            tags: vec![],
        };
        assert!(state.accept(duplicate).await.unwrap().is_none());
        assert_eq!(state.store.read().await.notifications.len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
