//! Dated habit check-ins for the shared Planner selection.
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
    pub id: String,
    pub editable: bool,
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
    pub async fn read_check_ins(
        &self,
        ids: Vec<String>,
        date: &str,
    ) -> Result<Vec<CheckIn>, Error> {
        parse_date(date)?;
        let date = date.to_owned();
        for id in &ids {
            validate_id(id)?;
        }
        self.transaction(move |connection| {
            let today = crate::current_date()?;
            ids.iter()
                .map(|id| read(connection, id, &date, &today))
                .collect()
        })
        .await
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
            // Validate after acquiring the write lock. Historical dates remain explicit.
            let today = crate::current_date()?;
            if date > today {
                return Err(Error::FutureCheckIn);
            }
            set(connection, &id, &date, completed)?;
            read(connection, &id, &date, &today)
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

fn read(
    connection: &mut SqliteConnection,
    id: &str,
    date: &str,
    local_date: &str,
) -> Result<CheckIn, Error> {
    let selected = parse_date(date)?;
    let cutoff = std::cmp::min(date, local_date);
    let recorded = check_ins::table
        .filter(check_ins::id.eq(id))
        .filter(check_ins::date.le(cutoff))
        .select(check_ins::date)
        .load::<String>(connection)?
        .into_iter()
        .map(|date| parse_date(&date))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let completed = recorded.contains(&selected);
    // An unfinished selected day does not break the previous day's ongoing streak.
    let mut cursor = if completed {
        Some(selected)
    } else {
        selected.previous_day()
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
        let day = selected
            .checked_sub(time::Duration::days(offset))
            .ok_or(Error::DateOverflow)?;
        days.push(Day {
            date: day.to_string(),
            completed: recorded.contains(&day),
        });
    }
    Ok(CheckIn {
        id: id.to_owned(),
        editable: date <= local_date,
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
