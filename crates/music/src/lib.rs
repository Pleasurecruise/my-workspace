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
    #[error(
        "Spotify returned 429 for read liked songs. Retry in {retry_after_secs} seconds. A personal Client ID in Settings can reduce shared quota delays."
    )]
    SpotifyRateLimited { retry_after_secs: u64 },
    #[error(
        "Spotify Web API quota is exhausted. Retry in {retry_after_secs} seconds or after the quota resets."
    )]
    SpotifyQuotaExhausted { retry_after_secs: u64 },
    #[error("Music provider authentication failed: {0}")]
    Authentication(String),
    #[error("Music provider request failed: {0}")]
    Request(#[source] reqwest::Error),
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

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        // QQ login and media URLs can carry credentials in their query strings.
        Self::Request(error.without_url())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_errors_omit_login_and_media_credentials() {
        let request =
            reqwest::Response::from(http::Response::builder().status(401).body("").unwrap())
                .error_for_status()
                .unwrap_err()
                .with_url(
                    "https://qqmusic.qq.com/audio?uin=synthetic-user&vkey=synthetic-secret"
                        .parse()
                        .unwrap(),
                );
        let error = Error::from(request);
        for message in [
            error.to_string(),
            format!("{error:?}"),
            std::error::Error::source(&error).unwrap().to_string(),
        ] {
            assert!(message.contains("401"));
            assert!(!message.contains("synthetic-user"));
            assert!(!message.contains("synthetic-secret"));
            assert!(!message.contains("qqmusic.qq.com"));
        }
    }
}
