use super::{Provider, Session};
use crate::{CredentialError, Stored};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Accounts {
    pub sessions: BTreeMap<String, Session>,
    pub bindings: BTreeMap<String, String>,
}

impl Accounts {
    pub fn insert(&mut self, session: Session) -> Result<(), CredentialError> {
        session.validate()?;
        let Session::Mihoyo { account_id, .. } = &session else {
            return Err(CredentialError::InvalidStore);
        };
        if self.sessions.is_empty() && self.bindings.is_empty() {
            for game in ["genshin", "starRail", "zzz"] {
                self.bindings.insert(game.into(), account_id.clone());
            }
        }
        self.sessions.insert(account_id.clone(), session);
        Ok(())
    }

    pub fn select(&mut self, game: &str, id: Option<&str>) -> Result<(), CredentialError> {
        if !matches!(game, "genshin" | "starRail" | "zzz") {
            return Err(CredentialError::InvalidValue(
                "miHoYo game",
                "unsupported game",
            ));
        }
        match id {
            Some(id) if self.sessions.contains_key(id) => {
                self.bindings.insert(game.into(), id.into());
            }
            Some(_) => {
                return Err(CredentialError::InvalidValue(
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

    pub fn remove(&mut self, id: &str) -> Result<(), CredentialError> {
        if self.sessions.remove(id).is_none() {
            return Err(CredentialError::InvalidValue(
                "miHoYo account",
                "account is not connected",
            ));
        }
        self.bindings.retain(|_, selected| selected != id);
        Ok(())
    }

    fn validate(&self) -> Result<(), CredentialError> {
        for (id, session) in &self.sessions {
            session.validate()?;
            let Session::Mihoyo { account_id, .. } = session else {
                return Err(CredentialError::InvalidStore);
            };
            if id != account_id {
                return Err(CredentialError::InvalidStore);
            }
        }
        for (game, id) in &self.bindings {
            if !matches!(game.as_str(), "genshin" | "starRail" | "zzz")
                || !self.sessions.contains_key(id)
            {
                return Err(CredentialError::InvalidStore);
            }
        }
        Ok(())
    }
}

fn lock() -> Result<File, CredentialError> {
    let directory = dirs::data_local_dir()
        .ok_or(CredentialError::StoreDataDirectory)?
        .join(crate::SERVICE);
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

fn load() -> Result<Accounts, CredentialError> {
    let accounts = match crate::store::read(Provider::Mihoyo.key())? {
        Stored::Missing => Accounts::default(),
        Stored::Ready(encoded) => serde_json::from_str::<Accounts>(&encoded)?,
    };
    accounts.validate()?;
    Ok(accounts)
}

pub fn read() -> Result<Accounts, CredentialError> {
    let _lock = lock()?;
    load()
}

pub fn update(
    change: impl FnOnce(&mut Accounts) -> Result<(), CredentialError>,
) -> Result<(), CredentialError> {
    let _lock = lock()?;
    let mut accounts = load()?;
    change(&mut accounts)?;
    accounts.validate()?;
    crate::store::save(Provider::Mihoyo.key(), &serde_json::to_string(&accounts)?)
}

#[cfg(test)]
#[path = "../../tests/unit/accounts.rs"]
mod tests;
