//! Local GBP expenses and calendar-month projections. No provider or credential boundary.
use diesel::prelude::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const MAX_PENCE: i64 = 99_999_999;
const MAX_TOTAL: i64 = 9_007_199_254_740_991;
const CATEGORIES: [&str; 7] = [
    "Coffee",
    "Subscriptions",
    "Eating out",
    "Groceries",
    "Transport",
    "Shopping",
    "Other",
];

diesel::table! {
    ledger_entries (id) {
        id -> Text,
        date -> Text,
        amount_pence -> BigInt,
        category -> Text,
        description -> Nullable<Text>,
        created_at -> BigInt,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Enter an amount between £0.01 and £999,999.99 with at most two decimal places")]
    Amount,
    #[error("Enter a category containing 1–40 characters without control characters")]
    Category,
    #[error("Enter a note of at most 500 characters without control characters")]
    Description,
    #[error("Choose a valid date in YYYY-MM-DD format")]
    Date,
    #[error("This expense no longer exists on the selected date")]
    MissingEntry,
    #[error("The expense total is too large to display accurately")]
    Total,
    #[error(transparent)]
    Database(#[from] vesper_database::Error),
    #[error("Expense storage operation failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("Expense storage task failed: {0}")]
    Task(String),
}

#[derive(Clone, Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = ledger_entries)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub date: String,
    pub amount_pence: i64,
    pub category: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTotal {
    pub category: String,
    pub amount_pence: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayTotal {
    pub date: String,
    pub amount_pence: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub date: String,
    pub month: String,
    pub entries: Vec<Entry>,
    pub day_total_pence: i64,
    pub month_total_pence: i64,
    pub categories: Vec<CategoryTotal>,
    pub days: Vec<DayTotal>,
    pub suggestions: Vec<String>,
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn read(&self, date: &str) -> Result<Snapshot, Error> {
        parse_date(date)?;
        let date = date.to_owned();
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path)?;
            connection.transaction(|connection| snapshot(connection, &date))
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

    pub async fn create(
        &self,
        date: &str,
        amount: &str,
        category: &str,
        description: Option<&str>,
    ) -> Result<Snapshot, Error> {
        let amount_pence = parse_amount(amount)?;
        let category = parse_category(category)?;
        let description = parse_description(description)?;
        self.mutate(date, move |connection, date| {
            let category = canonical_category(connection, &category)?;
            diesel::insert_into(ledger_entries::table)
                .values((
                    ledger_entries::id.eq(uuid::Uuid::new_v4().to_string()),
                    ledger_entries::date.eq(date),
                    ledger_entries::amount_pence.eq(amount_pence),
                    ledger_entries::category.eq(category),
                    ledger_entries::description.eq(description),
                    ledger_entries::created_at.eq(time::OffsetDateTime::now_utc().unix_timestamp()),
                ))
                .execute(connection)?;
            Ok(())
        })
        .await
    }

    pub async fn update(
        &self,
        date: &str,
        id: &str,
        amount: &str,
        category: &str,
        description: Option<&str>,
    ) -> Result<Snapshot, Error> {
        let amount_pence = parse_amount(amount)?;
        let category = parse_category(category)?;
        let change_description = description.is_some();
        let description = parse_description(description)?;
        let id = id.to_owned();
        self.mutate(date, move |connection, date| {
            let category = canonical_category(connection, &category)?;
            let count = diesel::update(
                ledger_entries::table
                    .filter(ledger_entries::id.eq(&id))
                    .filter(ledger_entries::date.eq(date)),
            )
            .set((
                ledger_entries::amount_pence.eq(amount_pence),
                ledger_entries::category.eq(category),
            ))
            .execute(connection)?;
            if count == 0 {
                return Err(Error::MissingEntry);
            }
            if change_description {
                diesel::update(ledger_entries::table.filter(ledger_entries::id.eq(&id)))
                    .set(ledger_entries::description.eq(description))
                    .execute(connection)?;
            }
            Ok(())
        })
        .await
    }

    pub async fn delete(&self, date: &str, id: &str) -> Result<Snapshot, Error> {
        let id = id.to_owned();
        self.mutate(date, move |connection, date| {
            let count = diesel::delete(
                ledger_entries::table
                    .filter(ledger_entries::id.eq(id))
                    .filter(ledger_entries::date.eq(date)),
            )
            .execute(connection)?;
            if count == 0 {
                return Err(Error::MissingEntry);
            }
            Ok(())
        })
        .await
    }

    async fn mutate(
        &self,
        date: &str,
        operation: impl FnOnce(&mut SqliteConnection, &str) -> Result<(), Error> + Send + 'static,
    ) -> Result<Snapshot, Error> {
        parse_date(date)?;
        let date = date.to_owned();
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path)?;
            connection.immediate_transaction(|connection| {
                operation(connection, &date)?;
                snapshot(connection, &date)
            })
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }
}

fn parse_date(value: &str) -> Result<time::Date, Error> {
    let date = time::Date::parse(
        value,
        &time::macros::format_description!("[year]-[month]-[day]"),
    )
    .map_err(|_| Error::Date)?;
    if !(1..=9999).contains(&date.year()) || date.to_string() != value {
        return Err(Error::Date);
    }
    Ok(date)
}

fn parse_amount(value: &str) -> Result<i64, Error> {
    let value = value.trim();
    let (pounds, fraction) = value.split_once('.').unwrap_or((value, ""));
    if pounds.is_empty()
        || !pounds.bytes().all(|c| c.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|c| c.is_ascii_digit())
        || (value.contains('.') && fraction.is_empty())
    {
        return Err(Error::Amount);
    }
    let pounds = pounds.parse::<i64>().map_err(|_| Error::Amount)?;
    let pennies = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().map_err(|_| Error::Amount)? * 10,
        _ => fraction.parse::<i64>().map_err(|_| Error::Amount)?,
    };
    let amount = pounds
        .checked_mul(100)
        .and_then(|p| p.checked_add(pennies))
        .ok_or(Error::Amount)?;
    if !(1..=MAX_PENCE).contains(&amount) {
        return Err(Error::Amount);
    }
    Ok(amount)
}

fn parse_category(value: &str) -> Result<String, Error> {
    if value.chars().any(char::is_control) {
        return Err(Error::Category);
    }
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if !(1..=40).contains(&value.chars().count()) {
        return Err(Error::Category);
    }
    Ok(value)
}

fn parse_description(value: Option<&str>) -> Result<Option<String>, Error> {
    let Some(value) = value else { return Ok(None) };
    if value.chars().any(char::is_control) || value.chars().count() > 500 {
        return Err(Error::Description);
    }
    let value = value.trim();
    Ok((!value.is_empty()).then(|| value.to_owned()))
}

fn canonical_category(connection: &mut SqliteConnection, category: &str) -> Result<String, Error> {
    let existing = ledger_entries::table
        .select(ledger_entries::category)
        .distinct()
        .order(ledger_entries::category.asc())
        .load::<String>(connection)?;
    Ok(CATEGORIES
        .iter()
        .copied()
        .chain(existing.iter().map(String::as_str))
        .find(|name| name.to_lowercase() == category.to_lowercase())
        .unwrap_or(category)
        .to_owned())
}

fn snapshot(connection: &mut SqliteConnection, date: &str) -> Result<Snapshot, Error> {
    let selected = parse_date(date)?;
    let first = selected.replace_day(1).map_err(|_| Error::Date)?;
    let count = selected.month().length(selected.year());
    let last = selected.replace_day(count).map_err(|_| Error::Date)?;
    let entries = ledger_entries::table
        .filter(ledger_entries::date.ge(first.to_string()))
        .filter(ledger_entries::date.le(last.to_string()))
        .order((ledger_entries::created_at.desc(), ledger_entries::id.asc()))
        .select(Entry::as_select())
        .load::<Entry>(connection)?;
    let mut days: BTreeMap<String, i64> = (0..count)
        .map(|offset| {
            (
                (first + time::Duration::days(i64::from(offset))).to_string(),
                0,
            )
        })
        .collect();
    let mut categories = BTreeMap::new();
    let mut total = 0_i64;
    for entry in &entries {
        total = total
            .checked_add(entry.amount_pence)
            .filter(|value| *value <= MAX_TOTAL)
            .ok_or(Error::Total)?;
        *categories.entry(entry.category.clone()).or_insert(0) += entry.amount_pence;
        *days.get_mut(&entry.date).ok_or(Error::Date)? += entry.amount_pence;
    }
    let used = ledger_entries::table
        .select(ledger_entries::category)
        .distinct()
        .load::<String>(connection)?;
    let mut suggestions: BTreeSet<String> = used.into_iter().collect();
    suggestions.extend(CATEGORIES.map(str::to_owned));
    suggestions.remove("Other");
    let mut categories: Vec<_> = categories
        .into_iter()
        .map(|(category, amount_pence)| CategoryTotal {
            category,
            amount_pence,
        })
        .collect();
    categories.sort_by(|a, b| {
        b.amount_pence
            .cmp(&a.amount_pence)
            .then(a.category.cmp(&b.category))
    });
    Ok(Snapshot {
        date: date.to_owned(),
        month: date[..7].to_owned(),
        day_total_pence: *days.get(date).ok_or(Error::Date)?,
        month_total_pence: total,
        entries: entries
            .into_iter()
            .filter(|entry| entry.date == date)
            .collect(),
        categories,
        days: days
            .into_iter()
            .map(|(date, amount_pence)| DayTotal { date, amount_pence })
            .collect(),
        suggestions: suggestions
            .into_iter()
            .chain(["Other".to_owned()])
            .collect(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/ledger.rs"]
mod tests;
