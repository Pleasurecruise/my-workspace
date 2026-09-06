use crate::{Account, Game, Pull};
use rusqlite::{Connection, OptionalExtension, params};
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

fn open(path: &Path) -> Result<Connection, String> {
    let parent = path.parent().ok_or("Invalid archive directory")?;
    std::fs::create_dir_all(parent).map_err(|_| "Could not create game archive directory")?;
    let connection = Connection::open(path)
        .map_err(|_| "Could not open game archive; existing data has been preserved")?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Could not configure archive locking")?;
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| "Invalid game archive")?;
    if version > 2 {
        return Err("Game archive was created by a newer Vesper version".to_owned());
    }
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;
        CREATE TABLE IF NOT EXISTS accounts (
            game TEXT NOT NULL, uid TEXT NOT NULL, name TEXT NOT NULL, region TEXT NOT NULL,
            role_id TEXT NOT NULL, synced_at INTEGER NOT NULL, PRIMARY KEY(game, uid));
        CREATE TABLE IF NOT EXISTS pulls (
            game TEXT NOT NULL, uid TEXT NOT NULL, id TEXT NOT NULL, pool TEXT NOT NULL,
            pool_name TEXT NOT NULL, item_id TEXT NOT NULL, name TEXT NOT NULL,
            rarity INTEGER NOT NULL CHECK(rarity BETWEEN 1 AND 6), time TEXT NOT NULL,
            is_free INTEGER, is_new INTEGER,
            PRIMARY KEY(game, uid, id), FOREIGN KEY(game, uid) REFERENCES accounts(game, uid));
        CREATE TABLE IF NOT EXISTS official_reports (
            game TEXT NOT NULL, uid TEXT NOT NULL, payload TEXT NOT NULL,
            PRIMARY KEY(game, uid), FOREIGN KEY(game, uid) REFERENCES accounts(game, uid));
        PRAGMA user_version=2;",
        )
        .map_err(|_| "Could not initialize game archive; existing data has been preserved")?;
    Ok(connection)
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
    let mut connection = open(path)?;
    let transaction = connection
        .transaction()
        .map_err(|_| "Could not start report transaction")?;
    let previous: Option<String> = transaction
        .query_row(
            "SELECT payload FROM official_reports WHERE game=?1 AND uid=?2",
            params![account.game.key(), account.uid],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Could not read previous Star Rail report")?;
    let previous = previous
        .map(|body| serde_json::from_str::<crate::StarRailReport>(&body))
        .transpose()
        .map_err(|_| "Previous Star Rail report is invalid; it was preserved")?;
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
        let current: HashSet<_> = pool.five_stars.iter().map(|star| star.id.clone()).collect();
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
    let body = serde_json::to_string(&report).map_err(|_| "Could not encode Star Rail report")?;
    transaction.execute("INSERT INTO accounts VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(game,uid) DO UPDATE SET name=excluded.name,region=excluded.region,role_id=excluded.role_id,synced_at=excluded.synced_at",
        params![account.game.key(),account.uid,account.name,account.region,account.role_id,crate::transport::now()]).map_err(|_| "Could not save Star Rail account")?;
    transaction.execute("INSERT INTO official_reports VALUES (?1,?2,?3) ON CONFLICT(game,uid) DO UPDATE SET payload=excluded.payload", params![account.game.key(),account.uid,body]).map_err(|_| "Could not save Star Rail report")?;
    transaction
        .commit()
        .map_err(|_| "Could not commit Star Rail report")?;
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
    let mut connection = open(path)?;
    let transaction = connection
        .transaction()
        .map_err(|_| "Could not start archive transaction")?;
    transaction.execute("INSERT INTO accounts VALUES (?1,?2,?3,?4,?5,?6)
        ON CONFLICT(game,uid) DO UPDATE SET name=excluded.name,region=excluded.region,role_id=excluded.role_id,synced_at=excluded.synced_at",
        params![account.game.key(), account.uid, account.name, account.region, account.role_id, crate::transport::now()])
        .map_err(|_| "Could not save archive account")?;
    let mut added = 0;
    {
        let mut insert = transaction.prepare("INSERT INTO pulls VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(game,uid,id) DO NOTHING").map_err(|_| "Could not prepare archive write")?;
        let mut existing = transaction.prepare("SELECT pool,NULLIF(item_id,''),rarity,time,is_free,is_new,name FROM pulls WHERE game=?1 AND uid=?2 AND id=?3").map_err(|_| "Could not verify existing archive")?;
        for pull in pulls {
            let mut rows = existing
                .query(params![account.game.key(), account.uid, pull.id])
                .map_err(|_| "Could not verify archived pull")?;
            if let Some(row) = rows.next().map_err(|_| "Could not verify archived pull")? {
                let saved: (
                    String,
                    Option<String>,
                    u8,
                    String,
                    Option<bool>,
                    Option<bool>,
                    String,
                ) = (
                    row.get(0).map_err(|_| "Invalid archived pool")?,
                    row.get(1).map_err(|_| "Invalid archived item")?,
                    row.get(2).map_err(|_| "Invalid archived rarity")?,
                    row.get(3).map_err(|_| "Invalid archived time")?,
                    row.get(4).map_err(|_| "Invalid archived free-pull flag")?,
                    row.get(5).map_err(|_| "Invalid archived new-item flag")?,
                    row.get(6).map_err(|_| "Invalid archived name")?,
                );
                let conflict = match (&saved.1, &pull.item_id) {
                    (Some(saved), Some(incoming)) => saved != incoming,
                    _ => saved.6 != pull.name,
                };
                let incoming = (
                    pull.pool.clone(),
                    saved.1.clone(),
                    pull.rarity,
                    pull.time.clone(),
                    pull.is_free,
                    pull.is_new,
                    saved.6.clone(),
                );
                if conflict || saved != incoming {
                    return Err(
                        "Provider history conflicts with an archived pull; archive was not changed"
                            .into(),
                    );
                }
                if saved.1.is_none() && pull.item_id.is_some() {
                    transaction
                        .execute(
                            "UPDATE pulls SET item_id=?1 WHERE game=?2 AND uid=?3 AND id=?4",
                            params![pull.item_id, account.game.key(), account.uid, pull.id],
                        )
                        .map_err(|_| "Could not complete archived item ID")?;
                }
            }
            added += insert
                .execute(params![
                    account.game.key(),
                    account.uid,
                    pull.id,
                    pull.pool,
                    pull.pool_name,
                    pull.item_id.as_deref().unwrap_or(""),
                    pull.name,
                    pull.rarity,
                    pull.time,
                    pull.is_free,
                    pull.is_new
                ])
                .map_err(|_| "Could not save pull records; transaction rolled back")?
                as u64;
        }
    }
    transaction
        .commit()
        .map_err(|_| "Could not commit pull archive")?;
    let mut result = summary(path, account.game, Some(&account.uid))?;
    result.added = added;
    Ok(result)
}

fn records(connection: &Connection, game: Game, uid: &str) -> Result<Vec<Pull>, String> {
    let mut statement = connection.prepare("SELECT id,pool,pool_name,NULLIF(item_id,''),name,rarity,time,is_free,is_new FROM pulls WHERE game=?1 AND uid=?2 ORDER BY time,length(id),id")
        .map_err(|_| "Could not read pull archive")?;
    let rows = statement
        .query_map(params![game.key(), uid], |row| {
            Ok(Pull {
                id: row.get(0)?,
                pool: row.get(1)?,
                pool_name: row.get(2)?,
                item_id: row.get(3)?,
                name: row.get(4)?,
                rarity: row.get(5)?,
                time: row.get(6)?,
                is_free: row.get(7)?,
                is_new: row.get(8)?,
            })
        })
        .map_err(|_| "Could not read pull archive")?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Game archive contains invalid records".to_owned())
}

pub fn summary(path: &Path, game: Game, requested_uid: Option<&str>) -> Result<Summary, String> {
    let connection = open(path)?;
    let mut statement = connection
        .prepare(
            "SELECT uid,name,synced_at FROM accounts WHERE game=?1 ORDER BY synced_at DESC,uid",
        )
        .map_err(|_| "Could not read archived accounts")?;
    let rows = statement
        .query_map([game.key()], |row| {
            Ok((
                ArchivedAccount {
                    uid: row.get(0)?,
                    name: row.get(1)?,
                },
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| "Could not read archived accounts")?;
    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Invalid archived account")?;
    let selected = match requested_uid {
        Some(uid) => Some(
            rows.iter()
                .find(|(account, _)| account.uid == uid)
                .ok_or("This account has no local archive")?,
        ),
        None => rows.first(),
    };
    let pulls = match selected {
        Some((account, _)) => records(&connection, game, &account.uid)?,
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
        let body: Option<String> = connection
            .query_row(
                "SELECT payload FROM official_reports WHERE game=?1 AND uid=?2",
                params![game.key(), account.uid],
                |row| row.get(0),
            )
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
