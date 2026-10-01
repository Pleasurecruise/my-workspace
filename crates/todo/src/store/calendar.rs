use super::{Store, normalized_text, read_list, save_items, todo_occurrences, todo_sources};
use crate::{Details, Error, Item, List, Subscription, parse_date, validate_date};
use diesel::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

struct CalendarSnapshot {
    view_url: String,
    loaded: std::time::Instant,
    items: Vec<Item>,
}

#[derive(Default)]
pub(super) struct CalendarCache {
    notion: Option<CalendarSnapshot>,
    codex: BTreeMap<String, (std::time::Instant, Vec<Item>)>,
}

const CALENDAR_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

impl Store {
    pub fn schedule_directory(&self) -> &Path {
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
        let file_guard = self.calendar_lock().await?;
        let directory = self.schedule_directory.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            // Keep the cross-process lock until installation finishes, even if the caller cancels.
            let _file = file_guard;
            std::fs::create_dir_all(&directory).map_err(|source| Error::Io {
                operation: "create",
                path: directory.clone(),
                source,
            })?;
            let mut installed = Vec::with_capacity(schedules.len());
            for (name, content) in schedules {
                let target = directory.join(name);
                install_schedule(&target, content.as_bytes()).map_err(|source| Error::Io {
                    operation: "install schedule (earlier files may already be installed)",
                    path: target.clone(),
                    source,
                })?;
                installed.push(target);
            }
            Ok(installed)
        })
        .await;
        match outcome {
            Ok(result) => result,
            Err(error) => Err(Error::Task(error.to_string())),
        }
    }

    // Coordinates the API read and commit with configuration writes in both Desktop and CLI.
    async fn calendar_lock(&self) -> Result<std::fs::File, Error> {
        let path = self.path.with_extension("calendar.lock");
        tokio::task::spawn_blocking(move || {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(Error::CalendarLock)?;
            }
            let mut options = std::fs::OpenOptions::new();
            options.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let lock = options.open(path).map_err(Error::CalendarLock)?;
            lock.lock().map_err(Error::CalendarLock)?;
            Ok(lock)
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

    pub async fn configure_notion(
        &self,
        configuration: crate::notion::configuration::Configuration,
    ) -> Result<(), Error> {
        let mut cache = self.calendar_read.lock().await;
        let _file = self.calendar_lock().await?;
        crate::notion::configuration::save(configuration)?;
        cache.notion = None;
        Ok(())
    }

    pub async fn read_codex(&self) -> Result<Subscription, Error> {
        self.transaction(|connection| {
            let enabled = todo_sources::table
                .find("codex-resets")
                .select(todo_sources::enabled)
                .first::<bool>(connection)
                .optional()?
                .unwrap_or(false);
            Ok(Subscription { enabled })
        })
        .await
    }

    pub async fn save_codex(&self, configuration: Subscription) -> Result<(), Error> {
        let mut cache = self.calendar_read.lock().await;
        let file = self.calendar_lock().await?;
        self.transaction(move |connection| {
            let _file = file;
            diesel::insert_into(todo_sources::table)
                .values((
                    todo_sources::name.eq("codex-resets"),
                    todo_sources::enabled.eq(configuration.enabled),
                ))
                .on_conflict(todo_sources::name)
                .do_update()
                .set(todo_sources::enabled.eq(configuration.enabled))
                .execute(connection)?;
            Ok(())
        })
        .await?;
        cache.codex.clear();
        Ok(())
    }

    pub async fn sync_calendar(&self, date: &str) -> Result<List, Error> {
        self.read_calendar(date, true).await
    }

    pub async fn read_calendar(&self, date: &str, refresh: bool) -> Result<List, Error> {
        validate_date(date)?;
        let mut cache = self.calendar_read.lock().await;
        let file_guard = self.calendar_lock().await?;
        let CalendarCache { notion, codex } = &mut *cache;
        let notion_read = async {
            let configuration = crate::notion::configuration::read()?;
            let remote = match &configuration {
                vault::Stored::Ready(configuration) => {
                    read_notion(notion, configuration, date, refresh).await?
                }
                vault::Stored::Missing => Vec::new(),
            };
            let current = crate::notion::configuration::read()?;
            let unchanged = match (&configuration, &current) {
                (vault::Stored::Ready(first), vault::Stored::Ready(second)) => {
                    first.view_url == second.view_url
                }
                (vault::Stored::Missing, vault::Stored::Missing) => true,
                _ => false,
            };
            if !unchanged {
                return Err(Error::Notion(
                    "configuration changed during the request; refresh again".into(),
                ));
            }
            Ok::<_, Error>(remote)
        };
        let codex_read = async {
            let enabled = self.read_codex().await?.enabled;
            if !enabled {
                codex.clear();
                return Ok(Vec::new());
            }
            let reusable = !refresh
                && codex
                    .get(date)
                    .is_some_and(|(loaded, _)| loaded.elapsed() < CALENDAR_CACHE_TTL);
            if !reusable {
                let items =
                    crate::codex::read(date, jiff::tz::TimeZone::system(), crate::codex::ENDPOINT)
                        .await?;
                if codex.len() >= 32 {
                    codex.clear();
                }
                codex.insert(date.to_owned(), (std::time::Instant::now(), items));
            }
            Ok::<_, Error>(
                codex
                    .get(date)
                    .map(|(_, items)| items.clone())
                    .unwrap_or_default(),
            )
        };
        let (result, codex) = tokio::join!(notion_read, codex_read);
        self.reconcile_calendar(date, result, codex, file_guard)
            .await
    }

    async fn reconcile_calendar(
        &self,
        date: &str,
        notion: Result<Vec<Item>, Error>,
        codex: Result<Vec<Item>, Error>,
        file_guard: std::fs::File,
    ) -> Result<List, Error> {
        let mut local = self.sync_schedule(date).await?;
        let mut errors = Vec::new();
        for (prefix, result) in [("notion:", notion), ("codex:", codex)] {
            match result {
                Ok(remote) => {
                    local = self
                        .replace_remote(
                            date,
                            prefix,
                            remote,
                            file_guard.try_clone().map_err(Error::CalendarLock)?,
                        )
                        .await?;
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
        local.sync_error = (!errors.is_empty()).then(|| errors.join("; "));
        Ok(local)
    }

    async fn replace_remote(
        &self,
        date: &str,
        prefix: &'static str,
        remote: Vec<Item>,
        file_guard: std::fs::File,
    ) -> Result<List, Error> {
        let date = date.to_owned();
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            // Cancellation must not release the calendar lock before this commit finishes.
            let _file = file_guard;
            let mut connection = database::open(&path)?;
            connection.immediate_transaction(move |connection| {
                let mut list = read_list(connection, &date)?;
                let removed: BTreeSet<String> = todo_occurrences::table
                    .filter(todo_occurrences::date.eq(&date))
                    .select(todo_occurrences::key)
                    .load::<String>(connection)?
                    .into_iter()
                    .collect();
                let carried: BTreeSet<String> = todo_occurrences::table
                    .filter(todo_occurrences::date.le(&date))
                    .filter(todo_occurrences::key.like(format!("rollover:{prefix}%")))
                    .select(todo_occurrences::key)
                    .load::<String>(connection)?
                    .into_iter()
                    .collect();
                let rollover: BTreeSet<String> = list
                    .items
                    .iter()
                    .filter(|item| item.rollover)
                    .map(|item| item.id.clone())
                    .collect();
                let completed: BTreeSet<String> = list
                    .items
                    .iter()
                    .filter(|item| item.completed)
                    .map(|item| item.id.clone())
                    .collect();
                let positions: BTreeMap<String, usize> = list
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (item.id.clone(), index))
                    .collect();
                list.items.retain(|item| {
                    !item.id.starts_with(prefix)
                        || (item.completed && carried.contains(&format!("rollover:{}", item.id)))
                });
                for mut item in remote {
                    if removed.contains(&item.id)
                        || carried.contains(&format!("rollover:{}", item.id))
                    {
                        continue;
                    }
                    item.completed = completed.contains(&item.id);
                    item.rollover = rollover.contains(&item.id);
                    list.items.push(item);
                }
                list.items
                    .sort_by_key(|item| positions.get(&item.id).copied().unwrap_or(usize::MAX));
                save_items(connection, &list)?;
                Ok(list)
            })
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

    pub async fn sync_schedule(&self, date: &str) -> Result<List, Error> {
        let parsed_date = parse_date(date)?;
        let date = date.to_owned();
        let schedules = self.load_schedules().await?;
        self.transaction(move |connection| {
            let mut imported: BTreeSet<String> = todo_occurrences::table
                .filter(todo_occurrences::date.eq(&date))
                .select(todo_occurrences::key)
                .load::<String>(connection)?
                .into_iter()
                .collect();
            let mut list = read_list(connection, &date)?;
            let mut changed = false;
            for (path, content) in schedules {
                let name = schedule_name(&path)?;
                let occurrences =
                    crate::schedule::occurrences(&content, parsed_date).map_err(|message| {
                        Error::ScheduleParse {
                            path: path.clone(),
                            message,
                        }
                    })?;
                for mut occurrence in occurrences {
                    normalized_text(&occurrence.text)?;
                    let key = format!("{name}:{}", occurrence.key);
                    if !imported.insert(key.clone()) {
                        continue;
                    }
                    occurrence.details.calendar = name.clone();
                    list.items.push(Item {
                        id: uuid::Uuid::new_v4().to_string(),
                        text: occurrence.text,
                        description: occurrence.details.description,
                        completed: false,
                        rollover: false,
                        source_owned: false,
                        details: Some(Details {
                            calendar: occurrence.details.calendar,
                            start_date: occurrence.details.start_date,
                            start_time: occurrence.details.start_time,
                            end_date: occurrence.details.end_date,
                            end_time: occurrence.details.end_time,
                            location: occurrence.details.location,
                        }),
                    });
                    diesel::insert_into(todo_occurrences::table)
                        .values((
                            todo_occurrences::date.eq(&date),
                            todo_occurrences::key.eq(key),
                        ))
                        .execute(connection)?;
                    changed = true;
                }
            }
            if changed {
                save_items(connection, &list)?;
            }
            Ok(list)
        })
        .await
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
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ics"))
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
}

// Own the remote snapshot lifecycle independently of dated SQLite projections.
async fn read_notion(
    cache: &mut Option<CalendarSnapshot>,
    configuration: &crate::notion::configuration::Configuration,
    date: &str,
    refresh: bool,
) -> Result<Vec<Item>, Error> {
    let reusable = cache.as_ref().is_some_and(|snapshot| {
        !refresh
            && snapshot.view_url == configuration.view_url
            && snapshot.loaded.elapsed() < CALENDAR_CACHE_TTL
    });
    if !reusable {
        let items = crate::notion::read(configuration).await?;
        *cache = Some(CalendarSnapshot {
            view_url: configuration.view_url.clone(),
            loaded: std::time::Instant::now(),
            items,
        });
    }
    Ok(cache
        .as_ref()
        .into_iter()
        .flat_map(|snapshot| &snapshot.items)
        .filter(|item| {
            item.details.as_ref().is_some_and(|details| {
                date >= details.start_date.as_str()
                    && date <= details.end_date.as_deref().unwrap_or(&details.start_date)
            })
        })
        .cloned()
        .collect())
}

// Stage complete bytes before replacing a schedule. Temporary files have no .ics extension,
// so an interrupted installation cannot become a schedule source on the next sync.
fn install_schedule(
    path: &std::path::Path,
    mut content: impl std::io::Read,
) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("schedule path has no parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut content, temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn schedule_name(path: &Path) -> Result<String, Error> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ics"))
    {
        return Err(Error::InvalidScheduleSource(path.to_owned()));
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidScheduleSource(path.to_owned()))
}

#[cfg(test)]
#[path = "../../tests/unit/calendar.rs"]
mod tests;
