use crate::{Calendar, Details, Error, Item, List, MAX_TEXT_LENGTH, parse_date, validate_date};
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tokio::sync::Mutex;

const APPLICATION_IDENTIFIER: &str = "me.you-find.vesper";
const FILE_NAME: &str = "todos.json";
const SCHEDULE_DIRECTORY_NAME: &str = "ics";

pub struct Store {
    path: PathBuf,
    schedule_directory: PathBuf,
    operation: Mutex<()>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        let schedule_directory = path.with_file_name(SCHEDULE_DIRECTORY_NAME);
        Self {
            path,
            schedule_directory,
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

    pub fn schedule_directory(&self) -> &std::path::Path {
        &self.schedule_directory
    }

    pub async fn import_schedules(&self, sources: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
        let mut names = BTreeSet::new();
        let mut schedules = Vec::with_capacity(sources.len());
        for source in sources {
            let name = schedule_name(source)?;
            if !names.insert(name.to_lowercase()) {
                return Err(Error::DuplicateScheduleName(name));
            }
            let content = tokio::fs::read_to_string(source)
                .await
                .map_err(|source_error| Error::Io {
                    operation: "read",
                    path: source.clone(),
                    source: source_error,
                })?;
            crate::schedule::validate(&content).map_err(|message| Error::ScheduleParse {
                path: source.clone(),
                message,
            })?;
            schedules.push((name, content));
        }
        let _operation = self.operation.lock().await;
        let _file = self.lock_file().await?;
        tokio::fs::create_dir_all(&self.schedule_directory)
            .await
            .map_err(|source| Error::Io {
                operation: "create",
                path: self.schedule_directory.clone(),
                source,
            })?;
        let mut installed = Vec::with_capacity(schedules.len());
        for (name, content) in schedules {
            let target = self.schedule_directory.join(name);
            tokio::fs::write(&target, content)
                .await
                .map_err(|source| Error::Io {
                    operation: "write",
                    path: target.clone(),
                    source,
                })?;
            installed.push(target);
        }
        Ok(installed)
    }

    pub async fn sync_schedule(&self, date: &str) -> Result<List, Error> {
        validate_date(date)?;
        let _operation = self.operation.lock().await;
        let _file = self.lock_file().await?;
        let schedules = self.load_schedules().await?;
        if schedules.is_empty() {
            let calendar = self.load().await?;
            return Ok(List {
                date: date.to_owned(),
                items: calendar.days.get(date).cloned().unwrap_or_default(),
            });
        }
        let parsed_date = parse_date(date)?;
        let mut occurrences = Vec::new();
        for (path, content) in schedules {
            let source = schedule_name(&path)?;
            let parsed =
                crate::schedule::occurrences(&content, parsed_date).map_err(|message| {
                    Error::ScheduleParse {
                        path: path.clone(),
                        message,
                    }
                })?;
            occurrences.extend(parsed.into_iter().map(|mut occurrence| {
                occurrence.key = format!("{source}:{}", occurrence.key);
                occurrence.details.calendar = source.clone();
                occurrence
            }));
        }
        for occurrence in &occurrences {
            normalized_text(&occurrence.text)?;
        }

        let mut calendar = self.load().await?;
        let imported = calendar
            .imported_occurrences
            .entry(date.to_owned())
            .or_default();
        let items = calendar.days.entry(date.to_owned()).or_default();
        let mut changed = false;
        for occurrence in occurrences {
            let details = Details {
                calendar: occurrence.details.calendar,
                start_date: occurrence.details.start_date,
                start_time: occurrence.details.start_time,
                end_date: occurrence.details.end_date,
                end_time: occurrence.details.end_time,
                location: occurrence.details.location,
                description: occurrence.details.description,
            };
            if imported.insert(occurrence.key) {
                items.push(Item {
                    id: uuid::Uuid::new_v4().to_string(),
                    text: occurrence.text,
                    completed: false,
                    details: Some(details),
                });
                changed = true;
            }
        }
        let list = List {
            date: date.to_owned(),
            items: items.clone(),
        };
        if changed {
            self.persist(&calendar).await?;
        }
        Ok(list)
    }

    async fn load_schedules(&self) -> Result<Vec<(PathBuf, String)>, Error> {
        let mut directory = match tokio::fs::read_dir(&self.schedule_directory).await {
            Ok(directory) => directory,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(Error::Io {
                    operation: "read",
                    path: self.schedule_directory.clone(),
                    source,
                });
            }
        };
        let mut paths = Vec::new();
        while let Some(entry) = directory.next_entry().await.map_err(|source| Error::Io {
            operation: "read",
            path: self.schedule_directory.clone(),
            source,
        })? {
            let path = entry.path();
            if entry
                .file_type()
                .await
                .map_err(|source| Error::Io {
                    operation: "inspect",
                    path: path.clone(),
                    source,
                })?
                .is_file()
                && has_ics_extension(&path)
            {
                paths.push(path);
            }
        }
        paths.sort();
        let mut schedules = Vec::with_capacity(paths.len());
        for path in paths {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|source| Error::Io {
                    operation: "read",
                    path: path.clone(),
                    source,
                })?;
            schedules.push((path, content));
        }
        Ok(schedules)
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
                details: None,
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
        let temporary = self.path.with_extension("json.tmp");
        if !self.path.exists() && temporary.exists() {
            replace_file(&temporary, &self.path)
                .await
                .map_err(|source| Error::Io {
                    operation: "recover",
                    path: self.path.clone(),
                    source,
                })?;
        }
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
        let temporary = self.path.with_extension("json.tmp");
        tokio::fs::write(&temporary, content)
            .await
            .map_err(|source| Error::Io {
                operation: "write",
                path: temporary.clone(),
                source,
            })?;
        tokio::fs::OpenOptions::new()
            .read(true)
            .open(&temporary)
            .await
            .map_err(|source| Error::Io {
                operation: "open",
                path: temporary.clone(),
                source,
            })?
            .sync_all()
            .await
            .map_err(|source| Error::Io {
                operation: "sync",
                path: temporary.clone(),
                source,
            })?;
        replace_file(&temporary, &self.path)
            .await
            .map_err(|source| Error::Io {
                operation: "replace",
                path: self.path.clone(),
                source,
            })
    }
}

#[cfg(not(windows))]
async fn replace_file(temporary: &std::path::Path, path: &std::path::Path) -> std::io::Result<()> {
    tokio::fs::rename(temporary, path).await
}

#[cfg(windows)]
async fn replace_file(temporary: &std::path::Path, path: &std::path::Path) -> std::io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    tokio::fs::rename(temporary, path).await
}

pub fn shared_path() -> Result<PathBuf, Error> {
    dirs::data_dir()
        .map(|path| path.join(APPLICATION_IDENTIFIER).join(FILE_NAME))
        .ok_or(Error::DataDirectoryUnavailable)
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

fn schedule_name(path: &std::path::Path) -> Result<String, Error> {
    if !has_ics_extension(path) {
        return Err(Error::InvalidScheduleSource(path.to_path_buf()));
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidScheduleSource(path.to_path_buf()))
}

fn has_ics_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ics"))
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
