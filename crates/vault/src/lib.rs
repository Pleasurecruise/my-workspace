#[cfg(debug_assertions)]
mod environment;
mod store;

#[cfg(debug_assertions)]
pub use environment::{load_dev_environment, variables};
pub use store::{delete, read, save};

use serde::Serialize;
use serde::de::DeserializeOwned;

#[cfg(not(debug_assertions))]
const SERVICE: &str = database::APP_ID;

pub enum Stored<T> {
    Missing,
    Ready(T),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] database::Error),
    #[error("credential database operation failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("credential store coordination failed: {0}")]
    StoreIo(#[from] std::io::Error),
    #[error("credential store synchronization is unavailable")]
    StoreSynchronization,
    #[error("stored credential collection is invalid")]
    InvalidStore,
    #[error("system credential store failed: {0}")]
    Store(#[from] keyring::Error),
    #[error("stored credential is invalid: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("credential field {0} cannot be empty")]
    Empty(&'static str),
    #[error("credential field {0} must contain at least {1} characters")]
    TooShort(&'static str, usize),
    #[error("credential field {0} is invalid: {1}")]
    InvalidValue(&'static str, &'static str),
    #[error("development credential {0} is incomplete")]
    IncompleteDevelopment(&'static str),
    #[error("development credential {0} is not valid Unicode")]
    InvalidDevelopment(&'static str),
    #[cfg(debug_assertions)]
    #[error("could not load development credentials from {}; check the file syntax and permissions", path.display())]
    DevelopmentFile { path: std::path::PathBuf },
}

pub fn read_json<T: DeserializeOwned>(account: &str) -> Result<Stored<T>, Error> {
    match read(account)? {
        Stored::Ready(encoded) => Ok(Stored::Ready(serde_json::from_str(&encoded)?)),
        Stored::Missing => Ok(Stored::Missing),
    }
}

pub fn save_json<T: Serialize>(account: &str, value: &T) -> Result<(), Error> {
    save(account, &serde_json::to_string(value)?)
}
