use crate::{Account, Game, Pull};
use diesel::prelude::*;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub total: u64,
    pub free_pulls: u64,
    pub high_rarity: u64,
    pub rarities: [u64; 6],
    pub since_high_rarity: u64,
    pub average_interval: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedAccount {
    pub uid: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub game: Game,
    pub uid: Option<String>,
    pub accounts: Vec<ArchivedAccount>,
    pub total: u64,
    pub added: u64,
    pub synced_at: Option<i64>,
    pub first_record: Option<String>,
    pub last_record: Option<String>,
    pub pools: Vec<Pool>,
    pub recent: Vec<Pull>,
    pub official: Option<crate::StarRailReport>,
}

diesel::table! {
    game_accounts (game, uid) {
        game -> Text, uid -> Text, name -> Text, region -> Text, role_id -> Text, synced_at -> BigInt,
    }
}
diesel::table! {
    game_pulls (game, uid, id) {
        game -> Text, uid -> Text, id -> Text, pool -> Text, pool_name -> Text,
        item_id -> Nullable<Text>, name -> Text, rarity -> Integer, time -> Text,
        is_free -> Nullable<Bool>, is_new -> Nullable<Bool>,
    }
}
diesel::table! {
    game_reports (game, uid) { game -> Text, uid -> Text, payload -> Text, }
}

#[derive(Debug, thiserror::Error)]
enum StoreError {
    #[error("Game archive database operation failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("{0}")]
    Invalid(&'static str),
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = game_pulls)]
struct PullRow {
    game: String,
    uid: String,
    id: String,
    pool: String,
    pool_name: String,
    item_id: Option<String>,
    name: String,
    rarity: i32,
    time: String,
    is_free: Option<bool>,
    is_new: Option<bool>,
}

fn save_account(
    connection: &mut SqliteConnection,
    account: &Account,
) -> Result<(), diesel::result::Error> {
    let fields = (
        game_accounts::name.eq(&account.name),
        game_accounts::region.eq(&account.region),
        game_accounts::role_id.eq(&account.role_id),
        game_accounts::synced_at.eq(crate::transport::now()),
    );
    diesel::insert_into(game_accounts::table)
        .values((
            game_accounts::game.eq(account.game.key()),
            game_accounts::uid.eq(&account.uid),
            fields,
        ))
        .on_conflict((game_accounts::game, game_accounts::uid))
        .do_update()
        .set(fields)
        .execute(connection)?;
    Ok(())
}

pub(crate) fn save_official(
    path: &Path,
    account: &Account,
    mut report: crate::StarRailReport,
) -> Result<Summary, String> {
    if account.game != Game::StarRail || account.uid.is_empty() {
        return Err("Invalid Star Rail report account".into());
    }
    let mut pool_ids = HashSet::new();
    for pool in &report.pools {
        let mut ids = HashSet::new();
        if !pool_ids.insert(&pool.id)
            || !matches!(pool.id.as_str(), "1" | "2" | "11" | "12" | "21" | "22")
            || pool.five_stars.iter().any(|star| {
                star.id.is_empty() || star.name.trim().is_empty() || !ids.insert(&star.id)
            })
        {
            return Err("Invalid Star Rail report; the previous archive is unchanged.".into());
        }
    }
    let mut connection = vesper_database::open(path).map_err(|error| error.to_string())?;
    let added = connection
        .immediate_transaction::<_, StoreError, _>(|connection| {
            let previous = game_reports::table
                .find((account.game.key(), &account.uid))
                .select(game_reports::payload)
                .first::<String>(connection)
                .optional()?;
            let previous = previous
                .map(|body| serde_json::from_str::<crate::StarRailReport>(&body))
                .transpose()
                .map_err(|_| {
                    StoreError::Invalid("Previous Star Rail report is invalid; it was preserved")
                })?;
            let mut added = 0;
            for pool in &mut report.pools {
                let older = previous
                    .as_ref()
                    .and_then(|report| report.pools.iter().find(|older| older.id == pool.id));
                let older_ids: HashSet<_> = older
                    .into_iter()
                    .flat_map(|pool| pool.five_stars.iter().map(|star| &star.id))
                    .collect();
                added += pool
                    .five_stars
                    .iter()
                    .filter(|star| !older_ids.contains(&star.id))
                    .count() as u64;
                let current: HashSet<_> =
                    pool.five_stars.iter().map(|star| star.id.clone()).collect();
                if let Some(older) = older {
                    pool.five_stars.extend(
                        older
                            .five_stars
                            .iter()
                            .filter(|star| !current.contains(&star.id))
                            .cloned(),
                    );
                }
            }
            if let Some(previous) = previous {
                let current: HashSet<_> = report.pools.iter().map(|pool| pool.id.clone()).collect();
                report.pools.extend(
                    previous
                        .pools
                        .into_iter()
                        .filter(|pool| !current.contains(&pool.id)),
                );
            }
            let body = serde_json::to_string(&report)
                .map_err(|_| StoreError::Invalid("Could not encode Star Rail report"))?;
            save_account(connection, account)?;
            diesel::insert_into(game_reports::table)
                .values((
                    game_reports::game.eq(account.game.key()),
                    game_reports::uid.eq(&account.uid),
                    game_reports::payload.eq(&body),
                ))
                .on_conflict((game_reports::game, game_reports::uid))
                .do_update()
                .set(game_reports::payload.eq(&body))
                .execute(connection)?;
            Ok(added)
        })
        .map_err(|error| error.to_string())?;
    let mut summary = summary(path, account.game, Some(&account.uid))?;
    summary.added = added;
    Ok(summary)
}

pub fn merge(path: &Path, account: &Account, pulls: &[Pull]) -> Result<Summary, String> {
    if account.uid.is_empty() {
        return Err("Game response has no account ID".to_owned());
    }
    let mut unique = HashSet::new();
    for pull in pulls {
        for (field, value) in [("id", &pull.id), ("pool", &pull.pool), ("name", &pull.name)] {
            if value.trim().is_empty() {
                return Err(format!(
                    "Pull record is missing {field}; archive was not changed"
                ));
            }
        }
        match &pull.item_id {
            Some(id) if id.trim().is_empty() => {
                return Err("Invalid pull item ID; archive was not changed".into());
            }
            None if !matches!(account.game, Game::Genshin | Game::StarRail) => {
                return Err("Pull record is missing item ID; archive was not changed".into());
            }
            _ => {}
        }
        if !(1..=6).contains(&pull.rarity) {
            return Err("Invalid pull rarity; archive was not changed".to_owned());
        }
        time::PrimitiveDateTime::parse(
            &pull.time,
            time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
        )
        .map_err(|_| "Invalid pull date; archive was not changed")?;
        if !unique.insert(&pull.id) {
            return Err("Repeated pull ID in game response; archive was not changed".to_owned());
        }
    }
    let mut connection = vesper_database::open(path).map_err(|error| error.to_string())?;
    let added = connection
        .immediate_transaction::<_, StoreError, _>(|connection| {
            save_account(connection, account)?;
            let mut added = 0;
            for pull in pulls {
                let key = (account.game.key(), &account.uid, &pull.id);
                let saved = game_pulls::table
                    .find(key)
                    .first::<PullRow>(connection)
                    .optional()?;
                if let Some(saved) = saved {
                    let item_conflict = match (&saved.item_id, &pull.item_id) {
                        (Some(saved), Some(incoming)) => saved != incoming,
                        _ => saved.name != pull.name,
                    };
                    let record_conflict = saved.pool != pull.pool
                        || saved.rarity != i32::from(pull.rarity)
                        || saved.time != pull.time
                        || saved.is_free != pull.is_free
                        || saved.is_new != pull.is_new;
                    if item_conflict || record_conflict {
                        return Err(StoreError::Invalid(
                            "Provider history conflicts with an archived pull; archive was not changed",
                        ));
                    }
                    if saved.item_id.is_none() && pull.item_id.is_some() {
                        diesel::update(game_pulls::table.find(key))
                            .set(game_pulls::item_id.eq(&pull.item_id))
                            .execute(connection)?;
                    }
                    continue;
                }
                let row = PullRow {
                    game: account.game.key().into(),
                    uid: account.uid.clone(),
                    id: pull.id.clone(),
                    pool: pull.pool.clone(),
                    pool_name: pull.pool_name.clone(),
                    item_id: pull.item_id.clone(),
                    name: pull.name.clone(),
                    rarity: i32::from(pull.rarity),
                    time: pull.time.clone(),
                    is_free: pull.is_free,
                    is_new: pull.is_new,
                };
                added += diesel::insert_into(game_pulls::table)
                    .values(row)
                    .execute(connection)? as u64;
            }
            Ok(added)
        })
        .map_err(|error| error.to_string())?;
    let mut result = summary(path, account.game, Some(&account.uid))?;
    result.added = added;
    Ok(result)
}

fn records(connection: &mut SqliteConnection, game: Game, uid: &str) -> Result<Vec<Pull>, String> {
    let rows = game_pulls::table
        .filter(game_pulls::game.eq(game.key()))
        .filter(game_pulls::uid.eq(uid))
        .order((
            game_pulls::time.asc(),
            diesel::dsl::sql::<diesel::sql_types::Integer>("length(id)").asc(),
            game_pulls::id.asc(),
        ))
        .load::<PullRow>(connection)
        .map_err(|_| "Could not read pull archive")?;
    rows.into_iter()
        .map(|row| {
            let rarity = u8::try_from(row.rarity)
                .ok()
                .filter(|value| (1..=6).contains(value))
                .ok_or("Invalid archived rarity")?;
            Ok(Pull {
                id: row.id,
                pool: row.pool,
                pool_name: row.pool_name,
                item_id: row.item_id,
                name: row.name,
                rarity,
                time: row.time,
                is_free: row.is_free,
                is_new: row.is_new,
            })
        })
        .collect()
}

pub fn summary(path: &Path, game: Game, requested_uid: Option<&str>) -> Result<Summary, String> {
    let mut connection = vesper_database::open(path).map_err(|error| error.to_string())?;
    let rows = game_accounts::table
        .filter(game_accounts::game.eq(game.key()))
        .order((game_accounts::synced_at.desc(), game_accounts::uid.asc()))
        .select((
            game_accounts::uid,
            game_accounts::name,
            game_accounts::synced_at,
        ))
        .load::<(String, String, i64)>(&mut connection)
        .map_err(|_| "Could not read archived accounts")?
        .into_iter()
        .map(|(uid, name, synced)| (ArchivedAccount { uid, name }, synced))
        .collect::<Vec<_>>();
    let selected = match requested_uid {
        Some(uid) => Some(
            rows.iter()
                .find(|(account, _)| account.uid == uid)
                .ok_or("This account has no local archive")?,
        ),
        None => rows.first(),
    };
    let pulls = match selected {
        Some((account, _)) => records(&mut connection, game, &account.uid)?,
        None => Vec::new(),
    };
    let top = match game {
        Game::Arknights | Game::Endfield => 6,
        _ => 5,
    };
    let mut pools: BTreeMap<String, (Pool, u64, u64, bool)> = BTreeMap::new();
    for pull in &pulls {
        let (pool, distance, intervals, seen_top) =
            pools.entry(pull.pool.clone()).or_insert_with(|| {
                (
                    Pool {
                        id: pull.pool.clone(),
                        name: pull.pool_name.clone(),
                        total: 0,
                        free_pulls: 0,
                        high_rarity: 0,
                        rarities: [0; 6],
                        since_high_rarity: 0,
                        average_interval: None,
                    },
                    0,
                    0,
                    false,
                )
            });
        pool.total += 1;
        if let Some(count) = pool
            .rarities
            .get_mut(usize::from(pull.rarity).saturating_sub(1))
        {
            *count += 1;
        }
        if pull.is_free == Some(true) {
            pool.free_pulls += 1;
        }
        pool.since_high_rarity += 1;
        if pull.rarity == top {
            pool.high_rarity += 1;
            if *seen_top {
                *distance += pool.since_high_rarity;
                *intervals += 1;
            }
            *seen_top = true;
            pool.since_high_rarity = 0;
        }
    }
    let pools = pools
        .into_values()
        .map(|(mut pool, distance, intervals, _)| {
            if intervals > 0 {
                pool.average_interval = Some(distance as f64 / intervals as f64);
            }
            pool
        })
        .collect();
    let official = if let Some((account, _)) = selected {
        let body = game_reports::table
            .find((game.key(), &account.uid))
            .select(game_reports::payload)
            .first::<String>(&mut connection)
            .optional()
            .map_err(|_| "Could not read official pull report")?;
        body.map(|body| serde_json::from_str(&body))
            .transpose()
            .map_err(|_| "Invalid official pull report")?
    } else {
        None
    };
    Ok(Summary {
        game,
        uid: selected.map(|(account, _)| account.uid.clone()),
        synced_at: selected.map(|(_, synced)| *synced),
        accounts: rows.iter().map(|(account, _)| account.clone()).collect(),
        total: pulls.len() as u64,
        added: 0,
        first_record: pulls.first().map(|pull| pull.time.clone()),
        last_record: pulls.last().map(|pull| pull.time.clone()),
        pools,
        recent: pulls.into_iter().rev().take(20).collect(),
        official,
    })
}

#[cfg(test)]
#[path = "../tests/unit/archive.rs"]
mod tests;
