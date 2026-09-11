//! Local daily check-ins, identified by a stable Dashboard placement ID.
use crate::{Error, Store, parse_date};
use diesel::prelude::*;
use serde::Serialize;
use std::collections::BTreeSet;

diesel::table! {
    check_ins (id, date) {
        id -> Text,
        date -> Text,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckIn {
    pub date: String,
    pub completed: bool,
    pub streak: usize,
    pub total: usize,
    pub days: Vec<Day>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Day {
    pub date: String,
    pub completed: bool,
}

impl Store {
    pub async fn read_check_ins(&self, ids: Vec<String>) -> Result<Vec<CheckIn>, Error> {
        for id in &ids {
            validate_id(id)?;
        }
        self.transaction(move |connection| {
            let date = crate::current_date()?;
            ids.iter().map(|id| read(connection, id, &date)).collect()
        })
        .await
    }

    pub async fn read_check_in(&self, id: &str) -> Result<CheckIn, Error> {
        validate_id(id)?;
        let id = id.to_owned();
        let path = self.database_path().to_owned();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path)?;
            read(&mut connection, &id, &crate::current_date()?)
        })
        .await
        .map_err(|error| Error::Task(error.to_string()))?
    }

    pub async fn set_check_in(
        &self,
        id: &str,
        date: &str,
        completed: bool,
    ) -> Result<CheckIn, Error> {
        validate_id(id)?;
        parse_date(date)?;
        let id = id.to_owned();
        let date = date.to_owned();
        self.transaction(move |connection| {
            // Check after acquiring the write lock, including when a request spans midnight.
            if date != crate::current_date()? {
                return Err(Error::CheckInDateChanged);
            }
            set(connection, &id, &date, completed)?;
            read(connection, &id, &date)
        })
        .await
    }
}

fn validate_id(id: &str) -> Result<(), Error> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(Error::InvalidCheckIn);
    }
    Ok(())
}

fn set(
    connection: &mut SqliteConnection,
    id: &str,
    date: &str,
    completed: bool,
) -> Result<(), Error> {
    if completed {
        diesel::insert_into(check_ins::table)
            .values((check_ins::id.eq(id), check_ins::date.eq(date)))
            .on_conflict_do_nothing()
            .execute(connection)?;
    } else {
        diesel::delete(
            check_ins::table
                .filter(check_ins::id.eq(id))
                .filter(check_ins::date.eq(date)),
        )
        .execute(connection)?;
    }
    Ok(())
}

fn read(connection: &mut SqliteConnection, id: &str, date: &str) -> Result<CheckIn, Error> {
    let today = parse_date(date)?;
    let recorded = check_ins::table
        .filter(check_ins::id.eq(id))
        .filter(check_ins::date.le(date))
        .select(check_ins::date)
        .load::<String>(connection)?
        .into_iter()
        .map(|date| parse_date(&date))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let completed = recorded.contains(&today);
    // An unfinished today does not break yesterday's ongoing streak.
    let mut cursor = if completed {
        Some(today)
    } else {
        today.previous_day()
    };
    let mut streak = 0;
    while let Some(day) = cursor {
        if !recorded.contains(&day) {
            break;
        }
        streak += 1;
        cursor = day.previous_day();
    }
    let mut days = Vec::with_capacity(28);
    for offset in (0..28).rev() {
        let day = today
            .checked_sub(time::Duration::days(offset))
            .ok_or(Error::DateOverflow)?;
        days.push(Day {
            date: day.to_string(),
            completed: recorded.contains(&day),
        });
    }
    Ok(CheckIn {
        date: date.to_owned(),
        completed,
        streak,
        total: recorded.len(),
        days,
    })
}

#[cfg(test)]
#[path = "../tests/unit/checkin.rs"]
mod tests;
