use super::{Provider, Session};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File};
use vault::Stored;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Accounts {
    pub sessions: BTreeMap<String, Session>,
    pub bindings: BTreeMap<String, String>,
}

impl Accounts {
    pub fn insert(&mut self, session: Session) -> Result<(), vault::Error> {
        session.validate()?;
        let Session::Mihoyo { account_id, .. } = &session else {
            return Err(vault::Error::InvalidStore);
        };
        if self.sessions.is_empty() && self.bindings.is_empty() {
            for game in ["genshin", "starRail", "zzz"] {
                self.bindings.insert(game.into(), account_id.clone());
            }
        }
        self.sessions.insert(account_id.clone(), session);
        Ok(())
    }

    pub fn select(&mut self, game: &str, id: Option<&str>) -> Result<(), vault::Error> {
        if !matches!(game, "genshin" | "starRail" | "zzz") {
            return Err(vault::Error::InvalidValue(
                "miHoYo game",
                "unsupported game",
            ));
        }
        match id {
            Some(id) if self.sessions.contains_key(id) => {
                self.bindings.insert(game.into(), id.into());
            }
            Some(_) => {
                return Err(vault::Error::InvalidValue(
                    "miHoYo account",
                    "account is not connected",
                ));
            }
            None => {
                self.bindings.remove(game);
            }
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), vault::Error> {
        if self.sessions.remove(id).is_none() {
            return Err(vault::Error::InvalidValue(
                "miHoYo account",
                "account is not connected",
            ));
        }
        self.bindings.retain(|_, selected| selected != id);
        Ok(())
    }

    fn validate(&self) -> Result<(), vault::Error> {
        for (id, session) in &self.sessions {
            session.validate()?;
            let Session::Mihoyo { account_id, .. } = session else {
                return Err(vault::Error::InvalidStore);
            };
            if id != account_id {
                return Err(vault::Error::InvalidStore);
            }
        }
        for (game, id) in &self.bindings {
            if !matches!(game.as_str(), "genshin" | "starRail" | "zzz")
                || !self.sessions.contains_key(id)
            {
                return Err(vault::Error::InvalidStore);
            }
        }
        Ok(())
    }
}

fn lock() -> Result<File, vault::Error> {
    let directory = database::directory()?;
    std::fs::create_dir_all(&directory)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("games-mihoyo.lock"))?;
    file.lock()?;
    Ok(file)
}

fn load() -> Result<Accounts, vault::Error> {
    let accounts = match vault::read_json(Provider::Mihoyo.key())? {
        Stored::Missing => Accounts::default(),
        Stored::Ready(accounts) => accounts,
    };
    accounts.validate()?;
    Ok(accounts)
}

pub fn read() -> Result<Accounts, vault::Error> {
    let _lock = lock()?;
    load()
}

pub fn update(
    change: impl FnOnce(&mut Accounts) -> Result<(), vault::Error>,
) -> Result<(), vault::Error> {
    let _lock = lock()?;
    let mut accounts = load()?;
    change(&mut accounts)?;
    accounts.validate()?;
    vault::save_json(Provider::Mihoyo.key(), &accounts)
}

#[cfg(test)]
#[path = "../../tests/unit/session/mihoyo.rs"]
mod tests;
