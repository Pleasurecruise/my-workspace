pub mod credentials;
pub mod knowledge;
pub mod memos;
pub mod moment;

use credentials::ConsumerApi;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use vault::Stored;

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// An authenticated HTTP client for one consumer service.
struct Client {
    api_key: String,
    http: reqwest::Client,
}

impl Client {
    fn load(service: ConsumerApi) -> Result<Self, ApiError> {
        let api_key = match credentials::read(service)? {
            Stored::Ready(api_key) => api_key,
            Stored::Missing => return Err(ApiError::MissingCredentials(service.service())),
        };
        Ok(Self {
            api_key,
            http: reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()?,
        })
    }
}

/// Send a request and require a successful status; callers keep their own
/// success-code assertions and response envelopes.
async fn send(
    request: reqwest::RequestBuilder,
    operation: &'static str,
) -> Result<reqwest::Response, ApiError> {
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status { operation, status });
    }
    Ok(response)
}

#[derive(Debug)]
pub enum ApiError {
    Credentials(vault::Error),
    MissingCredentials(&'static str),
    Media(moment::MediaError),
    Store(cms::r2::StoreError),
    Request(reqwest::Error),
    Status {
        operation: &'static str,
        status: reqwest::StatusCode,
    },
    Rejected {
        operation: &'static str,
        message: String,
    },
    Protocol(String),
}

impl Display for ApiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credentials(source) => Display::fmt(source, formatter),
            Self::MissingCredentials(service) => {
                write!(formatter, "{service} API key is not configured")
            }
            Self::Media(source) => Display::fmt(source, formatter),
            Self::Store(source) => Display::fmt(source, formatter),
            Self::Request(source) => write!(formatter, "consumer API request failed: {source}"),
            Self::Status { operation, status } => {
                write!(formatter, "consumer API {operation} returned {status}")
            }
            Self::Rejected { operation, message } => {
                write!(formatter, "consumer API {operation} rejected: {message}")
            }
            Self::Protocol(message) => {
                write!(formatter, "consumer API returned invalid data: {message}")
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
            Self::Media(source) => Some(source),
            Self::MissingCredentials(..)
            | Self::Status { .. }
            | Self::Rejected { .. }
            | Self::Protocol(..) => None,
        }
    }
}

impl From<vault::Error> for ApiError {
    fn from(source: vault::Error) -> Self {
        Self::Credentials(source)
    }
}

impl From<cms::r2::StoreError> for ApiError {
    fn from(source: cms::r2::StoreError) -> Self {
        Self::Store(source)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(source: reqwest::Error) -> Self {
        Self::Request(source)
    }
}
