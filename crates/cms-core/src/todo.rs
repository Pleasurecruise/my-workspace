use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::Mutex;

const APPLICATION_IDENTIFIER: &str = "me.you-find.vesper";
const FILE_NAME: &str = "todos.json";
const MAX_TEXT_LENGTH: usize = 120;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub date: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Calendar {
    days: BTreeMap<String, Vec<Item>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("the operating-system application data directory is unavailable")]
    DataDirectoryUnavailable,
    #[error("could not determine the local date: {0}")]
    LocalDate(#[from] time::error::IndeterminateOffset),
    #[error("invalid Todo date {0}; expected YYYY-MM-DD")]
    InvalidDate(String),
    #[error("todo text cannot be empty")]
    EmptyText,
    #[error("todo text cannot exceed {MAX_TEXT_LENGTH} characters")]
    TextTooLong,
    #[error("todo item no longer exists")]
    MissingItem,
    #[error("could not parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not encode the Todo list: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("could not {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Todo storage path has no parent directory")]
    MissingParent,
    #[error("could not calculate the next local date")]
    DateOverflow,
    #[error("Todo storage task failed: {0}")]
    Task(String),
}

pub struct Store {
    path: PathBuf,
    operation: Mutex<()>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            operation: Mutex::new(()),
        }
    }

    pub fn shared() -> Result<Self, Error> {
        Ok(Self::new(shared_path()?))
    }

    pub async fn list(&self, date: &str) -> Result<List, Error> {
        validate_date(date)?;
        let operation_guard = self.operation.lock().await;
        let file_guard = self.lock_file().await?;
        let calendar = self.load().await?;
        let list = List {
            date: date.to_owned(),
            items: calendar.days.get(date).cloned().unwrap_or_default(),
        };
        drop(file_guard);
        drop(operation_guard);
        Ok(list)
    }

    pub async fn get(&self, date: &str, id: &str) -> Result<Item, Error> {
        self.list(date)
            .await?
            .items
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(Error::MissingItem)
    }

    pub async fn create(&self, date: &str, text: &str) -> Result<List, Error> {
        let text = normalized_text(text)?;
        self.mutate(date, |items| {
            items.push(Item {
                id: uuid::Uuid::new_v4().to_string(),
                text: text.to_owned(),
                completed: false,
            });
            Ok(())
        })
        .await
    }

    pub async fn update(&self, date: &str, id: &str, text: &str) -> Result<List, Error> {
        let text = normalized_text(text)?;
        self.mutate(date, |items| {
            find_item(items, id)?.text = text.to_owned();
            Ok(())
        })
        .await
    }

    pub async fn set_completed(
        &self,
        date: &str,
        id: &str,
        completed: bool,
    ) -> Result<List, Error> {
        self.mutate(date, |items| {
            find_item(items, id)?.completed = completed;
            Ok(())
        })
        .await
    }

    pub async fn delete(&self, date: &str, id: &str) -> Result<List, Error> {
        self.mutate(date, |items| {
            let original_len = items.len();
            items.retain(|item| item.id != id);
            if items.len() == original_len {
                return Err(Error::MissingItem);
            }
            Ok(())
        })
        .await
    }

    async fn mutate(
        &self,
        date: &str,
        mutation: impl FnOnce(&mut Vec<Item>) -> Result<(), Error>,
    ) -> Result<List, Error> {
        validate_date(date)?;
        let operation_guard = self.operation.lock().await;
        let file_guard = self.lock_file().await?;
        let mut calendar = self.load().await?;
        let items = calendar.days.entry(date.to_owned()).or_default();
        mutation(items)?;
        let list = List {
            date: date.to_owned(),
            items: items.clone(),
        };
        if items.is_empty() {
            calendar.days.remove(date);
        }
        self.persist(&calendar).await?;
        drop(file_guard);
        drop(operation_guard);
        Ok(list)
    }

    async fn load(&self) -> Result<Calendar, Error> {
        match tokio::fs::read_to_string(&self.path).await {
            Ok(content) => serde_json::from_str(&content).map_err(|source| Error::Parse {
                path: self.path.clone(),
                source,
            }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Calendar::default()),
            Err(source) => Err(Error::Io {
                operation: "read",
                path: self.path.clone(),
                source,
            }),
        }
    }

    async fn lock_file(&self) -> Result<std::fs::File, Error> {
        let parent = self.path.parent().ok_or(Error::MissingParent)?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|source| Error::Io {
                operation: "create",
                path: parent.to_path_buf(),
                source,
            })?;
        let lock_path = self.path.with_extension("lock");
        let outcome = tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&lock_path)
                .map_err(|source| Error::Io {
                    operation: "open",
                    path: lock_path.clone(),
                    source,
                })?;
            file.lock().map_err(|source| Error::Io {
                operation: "lock",
                path: lock_path,
                source,
            })?;
            Ok(file)
        })
        .await;
        match outcome {
            Ok(result) => result,
            Err(error) => Err(Error::Task(error.to_string())),
        }
    }

    async fn persist(&self, calendar: &Calendar) -> Result<(), Error> {
        let parent = self.path.parent().ok_or(Error::MissingParent)?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|source| Error::Io {
                operation: "create",
                path: parent.to_path_buf(),
                source,
            })?;
        let content = serde_json::to_vec_pretty(calendar)?;
        tokio::fs::write(&self.path, content)
            .await
            .map_err(|source| Error::Io {
                operation: "write",
                path: self.path.clone(),
                source,
            })
    }
}

pub fn shared_path() -> Result<PathBuf, Error> {
    dirs::data_dir()
        .map(|path| path.join(APPLICATION_IDENTIFIER).join(FILE_NAME))
        .ok_or(Error::DataDirectoryUnavailable)
}

pub fn current_date() -> Result<String, Error> {
    Ok(time::OffsetDateTime::now_local()?.date().to_string())
}

pub fn validate_date(date: &str) -> Result<(), Error> {
    let parsed = time::Date::parse(
        date,
        &time::macros::format_description!("[year]-[month]-[day]"),
    );
    if parsed.is_err() {
        return Err(Error::InvalidDate(date.to_owned()));
    }
    Ok(())
}

pub fn next_rollover_delay() -> Result<std::time::Duration, Error> {
    let now = time::OffsetDateTime::now_local()?;
    let tomorrow = now.date().next_day().ok_or(Error::DateOverflow)?;
    let midnight = tomorrow.midnight();
    let approximate = midnight.assume_offset(now.offset());
    let midnight_offset = time::UtcOffset::local_offset_at(approximate)?;
    rollover_delay_at(now, midnight_offset)
}

fn rollover_delay_at(
    now: time::OffsetDateTime,
    midnight_offset: time::UtcOffset,
) -> Result<std::time::Duration, Error> {
    let tomorrow = now.date().next_day().ok_or(Error::DateOverflow)?;
    let midnight = tomorrow.midnight();
    Ok((midnight.assume_offset(midnight_offset) - now).unsigned_abs())
}

fn normalized_text(text: &str) -> Result<&str, Error> {
    let text = text.trim();
    if text.is_empty() {
        Err(Error::EmptyText)
    } else if text.chars().count() > MAX_TEXT_LENGTH {
        Err(Error::TextTooLong)
    } else {
        Ok(text)
    }
}

fn find_item<'a>(items: &'a mut [Item], id: &str) -> Result<&'a mut Item, Error> {
    items
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or(Error::MissingItem)
}

#[cfg(test)]
#[path = "../tests/unit/todo.rs"]
mod tests;
