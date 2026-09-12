mod app_lock;
mod codex;
mod content;
#[cfg(debug_assertions)]
mod environment;
pub mod games;
mod music;
mod notifications;
mod notion;
mod publication;
mod store;
mod ugos;

pub use app_lock::{AppLock, app_lock, delete_app_lock, save_app_lock};
pub use codex::{CodexResets, codex_resets, save_codex_resets};
pub use content::{ConsumerApi, R2Credentials, consumer_api, r2, save_consumer_api, save_r2};
#[cfg(debug_assertions)]
pub use environment::load_dev_environment;
pub use music::{
    QqMusicCredentials, SpotifyCredentials, qq_music, save_qq_music, save_spotify, spotify,
};
pub use notifications::{NtfyConfig, ntfy, save_ntfy};
pub use notion::{NotionCalendar, notion_calendar, save_notion_calendar};
pub use publication::{TelegramCredentials, XCredentials, save_telegram, save_x, telegram, x};
pub use ugos::{UgosCredentials, save_ugos, save_ugos_certificate, ugos, ugos_certificate};

const SERVICE: &str = "me.you-find.vesper";

pub enum Stored<T> {
    Missing,
    Ready(T),
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error(transparent)]
    Database(#[from] vesper_database::Error),
    #[error("credential database operation failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("credential store coordination failed: {0}")]
    StoreIo(#[from] std::io::Error),
    #[error("the operating system did not provide a credential store data directory")]
    StoreDataDirectory,
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
    #[error("could not load development credentials from {}: {source}", path.display())]
    DevelopmentFile {
        path: std::path::PathBuf,
        source: dotenvy::Error,
    },
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
