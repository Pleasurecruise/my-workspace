mod lyrics;
mod qq;
mod spotify;

pub use lyrics::{Lyrics, LyricsLine};
pub use qq::{QqLogin, QqLoginStatus, QqMusic, QqQr};
pub use spotify::{
    Cover, Playback, PlaybackOrder, Spotify, Track, authenticate, playback_authorization,
    web_authorization,
};

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    Spotify,
    QqMusic,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Music provider authentication failed: {0}")]
    Authentication(String),
    #[error("Music provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Music provider returned {status} for {operation}")]
    Status {
        operation: &'static str,
        status: reqwest::StatusCode,
    },
    #[error("Music provider returned invalid data: {0}")]
    InvalidData(String),
    #[error("Music provider playback failed: {0}")]
    Playback(String),
    #[error("Music provider credentials could not be stored: {0}")]
    Credentials(#[from] vesper_credentials::CredentialError),
}

pub type Result<T> = std::result::Result<T, Error>;
