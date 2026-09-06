use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::file;
use crate::{CredentialError, SERVICE, Stored};

struct Store {
    path: PathBuf,
    entries: BTreeMap<String, String>,
    _lock: File,
}

impl Store {
    fn open(directory: &Path) -> Result<Self, CredentialError> {
        std::fs::create_dir_all(directory)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(directory.join("credentials.lock"))?;
        lock.lock()?;
        let path = directory.join("credentials.json");
        let entries = match file::read(&path)? {
            Stored::Ready(entries) => entries,
            Stored::Missing => BTreeMap::new(),
        };
        Ok(Self {
            path,
            entries,
            _lock: lock,
        })
    }

    fn save(&mut self, account: &str, value: Option<&str>) -> Result<(), CredentialError> {
        match value {
            Some(value) => {
                self.entries.insert(account.to_owned(), value.to_owned());
            }
            None => {
                self.entries.remove(account);
            }
        }
        file::save(&self.path, &self.entries)
    }
}

fn directory() -> Result<PathBuf, CredentialError> {
    dirs::data_local_dir()
        .map(|root| root.join(SERVICE))
        .ok_or(CredentialError::DevelopmentDataDirectory)
}

pub(crate) fn read(account: &str) -> Result<Stored<String>, CredentialError> {
    let mut store = Store::open(&directory()?)?;
    match store.entries.remove(account) {
        Some(value) => Ok(Stored::Ready(value)),
        None => Ok(Stored::Missing),
    }
}

pub(crate) fn save(account: &str, value: &str) -> Result<(), CredentialError> {
    Store::open(&directory()?)?.save(account, Some(value))
}

pub(crate) fn delete(account: &str) -> Result<(), CredentialError> {
    Store::open(&directory()?)?.save(account, None)
}

#[cfg(test)]
#[path = "../../tests/unit/development_store.rs"]
mod tests;
