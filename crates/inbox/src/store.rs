use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

const LIMIT: usize = 200;
pub(crate) const TOPIC: &str = "mail-summary";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub topic: String,
    pub source: String,
    pub title: Option<String>,
    pub message: String,
    pub timestamp: i64,
    pub tags: Vec<String>,
}

#[derive(Clone, Default)]
pub(crate) struct Snapshot {
    pub(crate) last_id: Option<String>,
    pub(crate) notifications: Vec<Notification>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct Message {
    pub(crate) id: String,
    pub(crate) time: i64,
    pub(crate) event: String,
    pub(crate) topic: String,
    pub(crate) title: Option<String>,
    pub(crate) message: Option<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
}

#[derive(Deserialize)]
struct Envelope {
    source: String,
    title: Option<String>,
    body: String,
}

pub(crate) struct Store {
    path: PathBuf,
    pub(crate) snapshot: RwLock<Result<Snapshot, String>>,
}

impl Store {
    pub(crate) fn new(path: PathBuf) -> Self {
        let snapshot = (|| {
            let mut connection = database::open(&path).map_err(|error| error.to_string())?;
            let last_id = notification_cursor::table
                .find(1)
                .select(notification_cursor::last_id)
                .first::<Option<String>>(&mut connection)
                .optional()
                .map_err(|error| error.to_string())?
                .flatten();
            let rows = notifications::table
                .order(notifications::position.asc())
                .load::<Row>(&mut connection)
                .map_err(|error| error.to_string())?;
            let notifications = rows
                .into_iter()
                .map(|row| {
                    Ok(Notification {
                        id: row.id,
                        topic: row.topic,
                        source: row.source,
                        title: row.title,
                        message: row.message,
                        timestamp: row.timestamp,
                        tags: serde_json::from_str(&row.tags)
                            .map_err(|_| "Invalid notification tags".to_owned())?,
                    })
                })
                .collect::<Result<_, String>>()?;
            Ok(Snapshot {
                last_id,
                notifications,
            })
        })();
        Self {
            path,
            snapshot: RwLock::new(snapshot),
        }
    }

    pub(crate) async fn accept(
        &self,
        message: Message,
    ) -> Result<Option<Vec<Notification>>, String> {
        if message.event != "message" {
            return Ok(None);
        }
        if message.topic != TOPIC {
            return Err("ntfy returned invalid notification metadata".to_owned());
        }
        if !(0..=8_640_000_000).contains(&message.time) {
            return Err("ntfy returned invalid notification metadata".to_owned());
        }
        let Some(body) = message.message.filter(|body| !body.trim().is_empty()) else {
            return Ok(None);
        };
        let envelope = serde_json::from_str::<Envelope>(&body).ok();
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
        if source.len() > 200 || body.len() > 500_000 {
            return Err("ntfy returned an oversized notification".to_owned());
        }
        if title.as_ref().is_some_and(|title| title.len() > 500) {
            return Err("ntfy returned an oversized notification".to_owned());
        }
        if message.tags.len() > 50 || message.tags.iter().any(|tag| tag.len() > 100) {
            return Err("ntfy returned an oversized notification".to_owned());
        }
        let mut state = self.snapshot.write().await;
        let store = state.as_mut().map_err(|error| error.clone())?;
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
        next.notifications.truncate(LIMIT);
        persist(&self.path, &next).await?;
        *store = next;
        Ok(Some(store.notifications.clone()))
    }

    pub(crate) async fn mark_read(&self, id: &str) -> Result<Vec<Notification>, String> {
        let mut state = self.snapshot.write().await;
        let store = state.as_mut().map_err(|error| error.clone())?;
        let mut next = store.clone();
        let original_length = next.notifications.len();
        next.notifications
            .retain(|notification| notification.id != id);
        if next.notifications.len() == original_length {
            return Ok(store.notifications.clone());
        }
        persist(&self.path, &next).await?;
        *store = next;
        Ok(store.notifications.clone())
    }
}

diesel::table! {
    notifications (id) {
        id -> Text,
        position -> Integer,
        topic -> Text,
        source -> Text,
        title -> Nullable<Text>,
        message -> Text,
        timestamp -> BigInt,
        tags -> Text,
    }
}
diesel::table! {
    notification_cursor (id) { id -> Integer, last_id -> Nullable<Text>, }
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = notifications)]
struct Row {
    id: String,
    position: i32,
    topic: String,
    source: String,
    title: Option<String>,
    message: String,
    timestamp: i64,
    tags: String,
}

async fn persist(path: &Path, store: &Snapshot) -> Result<(), String> {
    let rows = store
        .notifications
        .iter()
        .enumerate()
        .map(|(position, item)| {
            Ok(Row {
                id: item.id.clone(),
                position: i32::try_from(position).map_err(|error| error.to_string())?,
                topic: item.topic.clone(),
                source: item.source.clone(),
                title: item.title.clone(),
                message: item.message.clone(),
                timestamp: item.timestamp,
                tags: serde_json::to_string(&item.tags).map_err(|error| error.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let last_id = store.last_id.clone();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut connection = database::open(&path).map_err(|error| error.to_string())?;
        connection
            .immediate_transaction::<_, diesel::result::Error, _>(|connection| {
                diesel::delete(notifications::table).execute(connection)?;
                diesel::insert_into(notifications::table)
                    .values(&rows)
                    .execute(connection)?;
                diesel::insert_into(notification_cursor::table)
                    .values((
                        notification_cursor::id.eq(1),
                        notification_cursor::last_id.eq(&last_id),
                    ))
                    .on_conflict(notification_cursor::id)
                    .do_update()
                    .set(notification_cursor::last_id.eq(&last_id))
                    .execute(connection)?;
                Ok(())
            })
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
