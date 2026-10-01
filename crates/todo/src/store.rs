mod calendar;

use crate::{Details, Error, Item, List, MAX_TEXT_LENGTH, parse_date, validate_date};
use calendar::CalendarCache;
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

    async fn list(&self, date: &str) -> Result<List, Error> {
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
                let mut days: BTreeMap<String, DayProgress> = BTreeMap::new();
                for day in 1..=selected.month().length(selected.year()) {
                    let date = selected
                        .replace_day(day)
                        .map_err(|_| Error::InvalidDate(selected.to_string()))?
                        .to_string();
                    if date > end {
                        break;
                    }
                    days.insert(date, DayProgress::new());
                }
                for (date, completed) in tasks {
                    days.entry(date)
                        .or_insert_with(DayProgress::new)
                        .tasks_completed &= completed;
                }
                let checks = crate::checkin::read_checks(connection, &ids, &start, &end)?;
                for (date, id) in checks {
                    days.entry(date)
                        .or_insert_with(DayProgress::new)
                        .checked_habits
                        .insert(id);
                }
                Ok(days
                    .into_iter()
                    .filter_map(|(date, day)| {
                        (day.tasks_completed && day.checked_habits == ids).then_some(date)
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
                source_owned: false,
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
            item.ensure_open()?;
            if item.source_owned {
                return Err(Error::RemoteItem);
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
            let item = find_item(items, &id)?;
            item.ensure_open()?;
            item.rollover = rollover;
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
                if item.source_owned {
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
                    item.source_owned = false;
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
            let item = find_item(&mut list.items, &id)?;
            item.ensure_open()?;
            let remote = item.source_owned;
            list.items.retain(|item| item.id != id);
            if remote {
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

/// A Planner day counts as complete when every task on it is completed and every
/// selected habit has a check-in.
struct DayProgress {
    tasks_completed: bool,
    checked_habits: BTreeSet<String>,
}

impl DayProgress {
    fn new() -> Self {
        Self {
            tasks_completed: true,
            checked_habits: BTreeSet::new(),
        }
    }
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
            // Existing source-prefixed IDs are the persisted ownership contract.
            let source_owned = row.id.starts_with("notion:") || row.id.starts_with("codex:");
            Ok(Item {
                source_owned,
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

fn normalized_description(value: &str) -> Result<Option<String>, Error> {
    let value = value.trim();
    if value.chars().count() > 4000 {
        return Err(Error::DescriptionTooLong);
    }
    Ok((!value.is_empty()).then(|| value.to_owned()))
}

fn find_item<'a>(items: &'a mut [Item], id: &str) -> Result<&'a mut Item, Error> {
    items
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or(Error::MissingItem)
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
