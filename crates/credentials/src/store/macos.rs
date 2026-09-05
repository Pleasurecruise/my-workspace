use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Mutex;

use crate::{CredentialError, SERVICE, Stored};

const ACCOUNT: &str = "credentials";

static STORE: Mutex<CredentialStore<Keychain>> = Mutex::new(CredentialStore {
    backend: Keychain,
    cache: None,
});

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    entries: BTreeMap<String, String>,
}

trait Backend {
    fn read(&self, account: &str) -> Result<Stored<String>, CredentialError>;
    fn save(&self, value: &str) -> Result<(), CredentialError>;
}

struct Keychain;

impl Backend for Keychain {
    fn read(&self, account: &str) -> Result<Stored<String>, CredentialError> {
        super::read_entry(account)
    }

    fn save(&self, value: &str) -> Result<(), CredentialError> {
        keyring::Entry::new(SERVICE, ACCOUNT)?.set_password(value)?;
        Ok(())
    }
}

struct CredentialStore<B> {
    backend: B,
    cache: Option<([u8; 16], Credentials)>,
}

impl<B: Backend> CredentialStore<B> {
    fn load(&mut self, file: &mut File) -> Result<Credentials, CredentialError> {
        file.rewind()?;
        let mut bytes = Vec::new();
        file.take(17).read_to_end(&mut bytes)?;
        let revision = <[u8; 16]>::try_from(bytes.as_slice()).ok();
        if let Some((cached_revision, credentials)) = &self.cache
            && revision == Some(*cached_revision)
        {
            return Ok(credentials.clone());
        }
        self.cache = None;
        let credentials = match self.backend.read(ACCOUNT)? {
            Stored::Ready(encoded) => {
                serde_json::from_str(&encoded).map_err(|_| CredentialError::InvalidStore)?
            }
            Stored::Missing => Credentials::default(),
        };
        let revision = match revision {
            Some(revision) => revision,
            None => advance_revision(file)?,
        };
        self.cache = Some((revision, credentials.clone()));
        Ok(credentials)
    }

    fn persist(
        &mut self,
        file: &mut File,
        credentials: &Credentials,
    ) -> Result<(), CredentialError> {
        let encoded = serde_json::to_string(credentials)?;
        self.cache = None;
        let revision = advance_revision(file)?;
        self.backend.save(&encoded)?;
        self.cache = Some((revision, credentials.clone()));
        Ok(())
    }

    fn read(&mut self, path: &Path, account: &str) -> Result<Stored<String>, CredentialError> {
        let mut file = lock(path)?;
        match self.load(&mut file)?.entries.remove(account) {
            Some(value) => Ok(Stored::Ready(value)),
            None => Ok(Stored::Missing),
        }
    }

    fn save(
        &mut self,
        path: &Path,
        account: &str,
        value: Option<&str>,
    ) -> Result<(), CredentialError> {
        let mut file = lock(path)?;
        let mut credentials = self.load(&mut file)?;
        match value {
            Some(value) => {
                credentials
                    .entries
                    .insert(account.to_owned(), value.to_owned());
            }
            None => {
                credentials.entries.remove(account);
            }
        }
        self.persist(&mut file, &credentials)
    }
}

fn lock(path: &Path) -> Result<File, CredentialError> {
    let parent = path.parent().ok_or(CredentialError::StoreDataDirectory)?;
    std::fs::create_dir_all(parent)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

fn advance_revision(file: &mut File) -> Result<[u8; 16], CredentialError> {
    let revision = *uuid::Uuid::new_v4().as_bytes();
    file.rewind()?;
    file.set_len(0)?;
    file.write_all(&revision)?;
    file.sync_data()?;
    Ok(revision)
}

pub(super) fn read(account: &str) -> Result<Stored<String>, CredentialError> {
    let path = dirs::data_local_dir()
        .ok_or(CredentialError::StoreDataDirectory)?
        .join(SERVICE)
        .join("credentials.lock");
    STORE
        .lock()
        .map_err(|_| CredentialError::StoreSynchronization)?
        .read(&path, account)
}

pub(super) fn save(account: &str, value: Option<&str>) -> Result<(), CredentialError> {
    let path = dirs::data_local_dir()
        .ok_or(CredentialError::StoreDataDirectory)?
        .join(SERVICE)
        .join("credentials.lock");
    STORE
        .lock()
        .map_err(|_| CredentialError::StoreSynchronization)?
        .save(&path, account, value)
}

#[cfg(test)]
#[path = "../../tests/unit/store.rs"]
mod tests;
