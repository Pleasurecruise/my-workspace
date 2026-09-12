use crate::{Details, Error, Item, List, MAX_TEXT_LENGTH, Subscription, parse_date, validate_date};
use diesel::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

diesel::table! {
    todo_sources (name) {
        name -> Text,
        enabled -> Bool,
    }
}
diesel::table! {
    todo_items (date, id) {
        date -> Text,
        id -> Text,
        position -> Integer,
        text -> Text,
        completed -> Bool,
        rollover -> Bool,
        calendar -> Nullable<Text>,
        start_date -> Nullable<Text>,
        start_time -> Nullable<Text>,
        end_date -> Nullable<Text>,
        end_time -> Nullable<Text>,
        location -> Nullable<Text>,
        description -> Nullable<Text>,
    }
}
diesel::table! {
    todo_occurrences (date, key) {
        date -> Text,
        key -> Text,
    }
}
#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = todo_items)]
struct ItemRow {
    date: String,
    id: String,
    position: i32,
    text: String,
    completed: bool,
    rollover: bool,
    calendar: Option<String>,
    start_date: Option<String>,
    start_time: Option<String>,
    end_date: Option<String>,
    end_time: Option<String>,
    location: Option<String>,
    description: Option<String>,
}

struct CalendarSnapshot {
    view_url: String,
    loaded: std::time::Instant,
    items: Vec<Item>,
}

#[derive(Default)]
struct CalendarCache {
    notion: Option<CalendarSnapshot>,
    codex: BTreeMap<String, (std::time::Instant, Vec<Item>)>,
}

pub struct Store {
    path: PathBuf,
    schedule_directory: PathBuf,
    calendar_read: tokio::sync::Mutex<CalendarCache>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            schedule_directory: path.with_file_name("ics"),
            path,
            calendar_read: tokio::sync::Mutex::new(CalendarCache::default()),
        }
    }

    pub fn shared() -> Result<Self, Error> {
        let mut store = Self::new(vesper_database::shared_path()?);
        // ICS remains a user-managed input in its original application-data directory.
        store.schedule_directory = dirs::data_dir()
            .ok_or(Error::DataDirectoryUnavailable)?
            .join("me.you-find.vesper")
            .join("ics");
        Ok(store)
    }

    pub fn database_path(&self) -> &Path {
        &self.path
    }

    pub(crate) async fn transaction<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut SqliteConnection) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path)?;
            connection.immediate_transaction(operation)
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

    pub async fn list(&self, date: &str) -> Result<List, Error> {
        validate_date(date)?;
        let date = date.to_owned();
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path)?;
            connection.transaction(|connection| read_list(connection, &date))
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

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
        configuration: vesper_credentials::NotionCalendar,
    ) -> Result<(), Error> {
        let mut cache = self.calendar_read.lock().await;
        let _file = self.calendar_lock().await?;
        vesper_credentials::save_notion_calendar(configuration)?;
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
            let configuration = vesper_credentials::notion_calendar()?;
            let remote = match &configuration {
                vesper_credentials::Stored::Ready(configuration) => {
                    read_notion(notion, configuration, date, refresh).await?
                }
                vesper_credentials::Stored::Missing => Vec::new(),
            };
            let current = vesper_credentials::notion_calendar()?;
            let unchanged = match (&configuration, &current) {
                (
                    vesper_credentials::Stored::Ready(first),
                    vesper_credentials::Stored::Ready(second),
                ) => first.view_url == second.view_url,
                (vesper_credentials::Stored::Missing, vesper_credentials::Stored::Missing) => true,
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
                && codex.get(date).is_some_and(|(loaded, _)| {
                    loaded.elapsed() < std::time::Duration::from_secs(300)
                });
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
            let mut connection = vesper_database::open(&path)?;
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
                            path: PathBuf::from(&name),
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

    pub async fn read_days(&self, ids: Vec<String>, date: &str) -> Result<Vec<String>, Error> {
        let selected = parse_date(date)?;
        let start = selected
            .replace_day(1)
            .map_err(|_| Error::InvalidDate(date.into()))?
            .to_string();
        let end = selected
            .replace_day(selected.month().length(selected.year()))
            .map_err(|_| Error::InvalidDate(date.into()))?
            .to_string();
        let ids: BTreeSet<String> = ids.into_iter().collect();
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path)?;
            connection.transaction::<_, Error, _>(|connection| {
                let today = crate::current_date()?;
                let end = std::cmp::min(end, today);
                let tasks = todo_items::table
                    .filter(todo_items::date.ge(&start))
                    .filter(todo_items::date.le(&end))
                    .select((todo_items::date, todo_items::completed))
                    .load::<(String, bool)>(connection)?;
                let mut days: BTreeMap<String, (bool, BTreeSet<String>)> = BTreeMap::new();
                for (date, completed) in tasks {
                    let day = days.entry(date).or_insert_with(|| (true, BTreeSet::new()));
                    day.0 &= completed;
                }
                use crate::checkin::check_ins;
                let checks = check_ins::table
                    .filter(check_ins::date.ge(&start))
                    .filter(check_ins::date.le(&end))
                    .filter(check_ins::id.eq_any(&ids))
                    .select((check_ins::date, check_ins::id))
                    .load::<(String, String)>(connection)?;
                for (date, id) in checks {
                    days.entry(date)
                        .or_insert_with(|| (true, BTreeSet::new()))
                        .1
                        .insert(id);
                }
                Ok(days
                    .into_iter()
                    .filter_map(|(date, (tasks_done, checked))| {
                        (tasks_done && checked == ids).then_some(date)
                    })
                    .collect())
            })
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

    pub async fn get(&self, date: &str, id: &str) -> Result<Item, Error> {
        self.list(date)
            .await?
            .items
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(Error::MissingItem)
    }

    pub async fn create(
        &self,
        date: &str,
        text: &str,
        description: Option<&str>,
    ) -> Result<List, Error> {
        let text = normalized_text(text)?.to_owned();
        let description = normalized_description(description.unwrap_or(""))?;
        self.mutate(date, move |items| {
            items.push(Item {
                id: uuid::Uuid::new_v4().to_string(),
                text,
                description,
                completed: false,
                rollover: false,
                details: None,
            });
            Ok(())
        })
        .await
    }

    pub async fn update(
        &self,
        date: &str,
        id: &str,
        text: &str,
        description: Option<&str>,
    ) -> Result<List, Error> {
        let text = normalized_text(text)?.to_owned();
        let description = description.map(normalized_description).transpose()?;
        let id = id.to_owned();
        self.mutate(date, move |items| {
            let item = find_item(items, &id)?;
            if item.details.is_some() {
                return Err(Error::ImportedItem);
            }
            item.text = text;
            if let Some(description) = description {
                item.description = description;
            }
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
        let id = id.to_owned();
        self.mutate(date, move |items| {
            find_item(items, &id)?.completed = completed;
            Ok(())
        })
        .await
    }

    pub async fn set_rollover(&self, date: &str, id: &str, rollover: bool) -> Result<List, Error> {
        let id = id.to_owned();
        self.mutate(date, move |items| {
            find_item(items, &id)?.rollover = rollover;
            Ok(())
        })
        .await
    }

    /// Move opted-in unfinished tasks to the actual local day, never a browsed future date.
    pub async fn roll_over(&self, today: &str) -> Result<Vec<String>, Error> {
        validate_date(today)?;
        let today = today.to_owned();
        self.transaction(move |connection| {
            let dates = todo_items::table
                .filter(todo_items::date.lt(&today))
                .filter(todo_items::rollover.eq(true))
                .filter(todo_items::completed.eq(false))
                .select(todo_items::date)
                .distinct()
                .order(todo_items::date.asc())
                .load::<String>(connection)?;
            if dates.is_empty() {
                return Ok(Vec::new());
            }
            let mut lists = BTreeMap::new();
            let mut pending = Vec::new();
            for date in &dates {
                let mut list = read_list(connection, date)?;
                let mut retained = Vec::new();
                for item in list.items {
                    if item.rollover && !item.completed {
                        pending.push((date.clone(), item));
                    } else {
                        retained.push(item);
                    }
                }
                list.items = retained;
                lists.insert(date.clone(), list);
            }
            lists.insert(today.clone(), read_list(connection, &today)?);
            let mut carried = BTreeSet::new();
            for (source_date, mut item) in pending {
                if item.id.starts_with("notion:") || item.id.starts_with("codex:") {
                    if !carried.insert(item.id.clone()) {
                        continue;
                    }
                    // A multi-day event becomes a single local follow-up. Remove cached projections
                    // in the same transaction, including future dates and offline calendar snapshots.
                    let related = todo_items::table
                        .filter(todo_items::id.eq(&item.id))
                        .filter(todo_items::date.ge(&source_date))
                        .select(todo_items::date)
                        .distinct()
                        .load::<String>(connection)?;
                    for date in related {
                        if !lists.contains_key(&date) {
                            lists.insert(date.clone(), read_list(connection, &date)?);
                        }
                        let list = lists.get_mut(&date).ok_or(Error::InvalidRecord)?;
                        list.items.retain(|entry| {
                            if entry.id != item.id {
                                return true;
                            }
                            item.completed |= entry.completed;
                            entry.completed
                        });
                    }
                    diesel::insert_into(todo_occurrences::table)
                        .values((
                            todo_occurrences::date.eq(&source_date),
                            todo_occurrences::key.eq(format!("rollover:{}", item.id)),
                        ))
                        .on_conflict_do_nothing()
                        .execute(connection)?;
                    if item.completed {
                        continue;
                    }
                    item.id = format!("rollover:{}", uuid::Uuid::new_v4());
                }
                let destination = lists.get_mut(&today).ok_or(Error::InvalidRecord)?;
                if destination.items.iter().any(|entry| entry.id == item.id) {
                    return Err(Error::InvalidRecord);
                }
                destination.items.push(item);
            }
            for list in lists.values() {
                save_items(connection, list)?;
            }
            Ok(lists.into_keys().collect())
        })
        .await
    }

    pub async fn reorder(&self, date: &str, ids: Vec<String>) -> Result<List, Error> {
        self.mutate(date, move |items| {
            let positions: BTreeMap<&str, usize> = ids
                .iter()
                .enumerate()
                .map(|(index, id)| (id.as_str(), index))
                .collect();
            if ids.len() != items.len()
                || positions.len() != ids.len()
                || items
                    .iter()
                    .any(|item| !positions.contains_key(item.id.as_str()))
            {
                return Err(Error::InvalidOrder);
            }
            items.sort_by_key(|item| positions[item.id.as_str()]);
            Ok(())
        })
        .await
    }

    pub async fn delete(&self, date: &str, id: &str) -> Result<List, Error> {
        validate_date(date)?;
        let date = date.to_owned();
        let id = id.to_owned();
        self.transaction(move |connection| {
            let mut list = read_list(connection, &date)?;
            let original_len = list.items.len();
            list.items.retain(|item| item.id != id);
            if list.items.len() == original_len {
                return Err(Error::MissingItem);
            }
            if id.starts_with("notion:") || id.starts_with("codex:") {
                diesel::insert_into(todo_occurrences::table)
                    .values((
                        todo_occurrences::date.eq(&date),
                        todo_occurrences::key.eq(id),
                    ))
                    .on_conflict_do_nothing()
                    .execute(connection)?;
            }
            save_items(connection, &list)?;
            Ok(list)
        })
        .await
    }

    async fn mutate(
        &self,
        date: &str,
        mutation: impl FnOnce(&mut Vec<Item>) -> Result<(), Error> + Send + 'static,
    ) -> Result<List, Error> {
        validate_date(date)?;
        let date = date.to_owned();
        self.transaction(move |connection| {
            let mut list = read_list(connection, &date)?;
            mutation(&mut list.items)?;
            save_items(connection, &list)?;
            Ok(list)
        })
        .await
    }
}

// Own the remote snapshot lifecycle independently of dated SQLite projections.
async fn read_notion(
    cache: &mut Option<CalendarSnapshot>,
    configuration: &vesper_credentials::NotionCalendar,
    date: &str,
    refresh: bool,
) -> Result<Vec<Item>, Error> {
    let reusable = cache.as_ref().is_some_and(|snapshot| {
        !refresh
            && snapshot.view_url == configuration.view_url
            && snapshot.loaded.elapsed() < std::time::Duration::from_secs(300)
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

fn read_list(connection: &mut SqliteConnection, date: &str) -> Result<List, Error> {
    let rows = todo_items::table
        .filter(todo_items::date.eq(date))
        .order(todo_items::position.asc())
        .select(ItemRow::as_select())
        .load(connection)?;
    let items = rows
        .into_iter()
        .map(|row| {
            let details = match (row.calendar, row.start_date) {
                (Some(calendar), Some(start_date)) => Some(Details {
                    calendar,
                    start_date,
                    start_time: row.start_time,
                    end_date: row.end_date,
                    end_time: row.end_time,
                    location: row.location,
                }),
                (None, None) => None,
                _ => return Err(Error::InvalidRecord),
            };
            Ok(Item {
                id: row.id,
                text: row.text,
                description: row.description,
                completed: row.completed,
                rollover: row.rollover,
                details,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(List {
        sync_error: None,
        date: date.to_owned(),
        items,
    })
}

fn save_items(connection: &mut SqliteConnection, list: &List) -> Result<(), Error> {
    diesel::delete(todo_items::table.filter(todo_items::date.eq(&list.date)))
        .execute(connection)?;
    for (position, item) in list.items.iter().enumerate() {
        let row = ItemRow {
            date: list.date.clone(),
            id: item.id.clone(),
            position: i32::try_from(position).map_err(|_| Error::InvalidRecord)?,
            text: item.text.clone(),
            completed: item.completed,
            rollover: item.rollover,
            calendar: item
                .details
                .as_ref()
                .map(|details| details.calendar.clone()),
            start_date: item
                .details
                .as_ref()
                .map(|details| details.start_date.clone()),
            start_time: item
                .details
                .as_ref()
                .and_then(|details| details.start_time.clone()),
            end_date: item
                .details
                .as_ref()
                .and_then(|details| details.end_date.clone()),
            end_time: item
                .details
                .as_ref()
                .and_then(|details| details.end_time.clone()),
            location: item
                .details
                .as_ref()
                .and_then(|details| details.location.clone()),
            description: item.description.clone(),
        };
        diesel::insert_into(todo_items::table)
            .values(row)
            .execute(connection)?;
    }
    Ok(())
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

fn normalized_text(text: &str) -> Result<&str, Error> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Error::EmptyText);
    }
    if text.chars().count() > MAX_TEXT_LENGTH {
        return Err(Error::TextTooLong);
    }
    Ok(text)
}

fn find_item<'a>(items: &'a mut [Item], id: &str) -> Result<&'a mut Item, Error> {
    items
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or(Error::MissingItem)
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
#[path = "../tests/unit/store.rs"]
mod tests;

fn normalized_description(value: &str) -> Result<Option<String>, Error> {
    let value = value.trim();
    if value.chars().count() > 4000 {
        return Err(Error::DescriptionTooLong);
    }
    Ok((!value.is_empty()).then(|| value.to_owned()))
}
