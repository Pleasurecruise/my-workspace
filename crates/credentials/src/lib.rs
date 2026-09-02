mod app_lock;
mod consumer;
#[cfg(debug_assertions)]
mod development;
#[cfg(debug_assertions)]
mod development_storage;
mod ntfy;
mod qq;
mod r2;
mod social;
mod spotify;
mod ugos;

pub use app_lock::{AppLock, app_lock, delete_app_lock, save_app_lock};
pub use consumer::{ConsumerApi, consumer_api, save_consumer_api};
#[cfg(debug_assertions)]
pub use development::load_dev_environment;
pub use ntfy::{NtfyConfig, ntfy, save_ntfy};
pub use qq::{QqMusicCredentials, qq_music, save_qq_music};
pub use r2::{R2Credentials, r2, save_r2};
pub use social::{TelegramCredentials, XCredentials, save_telegram, save_x, telegram, x};
pub use spotify::{SpotifyCredentials, save_spotify, spotify};
pub use ugos::{UgosCredentials, save_ugos, save_ugos_certificate, ugos, ugos_certificate};

const SERVICE: &str = "me.you-find.vesper";

pub enum Stored<T> {
    Missing,
    Ready(T),
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
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
    #[error("could not load development credentials from {}: {source}", path.display())]
    DevelopmentFile {
        path: std::path::PathBuf,
        source: dotenvy::Error,
    },
    #[cfg(debug_assertions)]
    #[error("development credential storage failed at {}: {source}", path.display())]
    DevelopmentStorage {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[cfg(debug_assertions)]
    #[error("the operating system did not provide a development data directory")]
    DevelopmentDataDirectory,
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
