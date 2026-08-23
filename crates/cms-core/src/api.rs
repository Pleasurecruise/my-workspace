pub mod knowledge;
pub mod memos;
pub mod moment;

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug)]
pub enum ApiError {
    Credentials(vesper_credentials::CredentialError),
    MissingCredentials(&'static str),
    Store(crate::r2::StoreError),
    Request(reqwest::Error),
    Status {
        operation: &'static str,
        status: reqwest::StatusCode,
    },
    Protocol(String),
    InvalidMemoBody {
        key: String,
        source: std::string::FromUtf8Error,
    },
}

impl Display for ApiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credentials(source) => Display::fmt(source, formatter),
            Self::MissingCredentials(service) => {
                write!(formatter, "{service} API key is not configured")
            }
            Self::Store(source) => Display::fmt(source, formatter),
            Self::Request(source) => write!(formatter, "consumer API request failed: {source}"),
            Self::Status { operation, status } => {
                write!(formatter, "consumer API {operation} returned {status}")
            }
            Self::Protocol(message) => {
                write!(formatter, "consumer API returned invalid data: {message}")
            }
            Self::InvalidMemoBody { key, source } => {
                write!(formatter, "R2 memo {key} is not UTF-8: {source}")
            }
        }
    }
}

impl Error for ApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Credentials(source) => Some(source),
            Self::Store(source) => Some(source),
            Self::Request(source) => Some(source),
            Self::InvalidMemoBody { source, .. } => Some(source),
            Self::MissingCredentials(..) | Self::Status { .. } | Self::Protocol(..) => None,
        }
    }
}

impl From<vesper_credentials::CredentialError> for ApiError {
    fn from(source: vesper_credentials::CredentialError) -> Self {
        Self::Credentials(source)
    }
}

impl From<crate::r2::StoreError> for ApiError {
    fn from(source: crate::r2::StoreError) -> Self {
        Self::Store(source)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(source: reqwest::Error) -> Self {
        Self::Request(source)
    }
}
