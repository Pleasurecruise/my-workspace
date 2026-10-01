pub mod telegram;
mod text;
pub mod x;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use vault::Stored;

const MEMO_ORIGIN: &str = "https://memos.you-find.me/memo";

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoPublication {
    pub id: String,
    pub content: String,
    pub visibility: PublicationVisibility,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublicationVisibility {
    Public,
    Private,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PublicationProvider {
    Telegram,
    X,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPost {
    pub provider: PublicationProvider,
    pub external_id: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configured {
    pub telegram: bool,
    pub x: bool,
}

#[derive(Debug)]
pub enum PublishError {
    Credentials(vault::Error),
    MissingCredentials(&'static str),
    InvalidMemo(&'static str),
    Request(&'static str),
    Status {
        provider: &'static str,
        status: StatusCode,
    },
    Protocol(&'static str),
    Session(&'static str),
    Authorization(&'static str),
    XAuthorization(&'static str),
}

impl Display for PublishError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credentials(source) => Display::fmt(source, formatter),
            Self::MissingCredentials(provider) => {
                write!(formatter, "{provider} publication is not configured")
            }
            Self::InvalidMemo(message) => write!(formatter, "memo cannot be published: {message}"),
            Self::Request(provider) => write!(formatter, "{provider} publication request failed"),
            Self::Status { provider, status } => {
                write!(formatter, "{provider} publication returned {status}")
            }
            Self::Protocol(provider) => {
                write!(
                    formatter,
                    "{provider} returned an invalid publication response"
                )
            }
            Self::Session(message) => write!(formatter, "Telegram session failed: {message}"),
            Self::Authorization(message) => {
                write!(formatter, "Telegram authorization failed: {message}")
            }
            Self::XAuthorization(message) => write!(formatter, "X authorization failed: {message}"),
        }
    }
}

impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Credentials(source) => Some(source),
            Self::MissingCredentials(..)
            | Self::InvalidMemo(..)
            | Self::Request(..)
            | Self::Status { .. }
            | Self::Protocol(..)
            | Self::Session(..)
            | Self::Authorization(..)
            | Self::XAuthorization(..) => None,
        }
    }
}

impl From<vault::Error> for PublishError {
    fn from(source: vault::Error) -> Self {
        Self::Credentials(source)
    }
}

pub fn configured() -> Result<Configured, PublishError> {
    Ok(Configured {
        telegram: matches!(telegram::credentials::read()?, Stored::Ready(_)),
        x: matches!(x::credentials::read()?, Stored::Ready(_)),
    })
}

fn memo_url(memo: &MemoPublication) -> Result<String, PublishError> {
    if !matches!(memo.visibility, PublicationVisibility::Public) {
        return Err(PublishError::InvalidMemo(
            "only public memos can be published",
        ));
    }
    if memo.id.is_empty()
        || memo.id.len() > 128
        || !memo
            .id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(PublishError::InvalidMemo("the ID is invalid"));
    }
    if memo.content.trim().is_empty() {
        return Err(PublishError::InvalidMemo("the content is empty"));
    }
    Ok(format!("{MEMO_ORIGIN}/{}", memo.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_memo_id() {
        let result = memo_url(&MemoPublication {
            id: "../secret".to_owned(),
            content: "content".to_owned(),
            visibility: PublicationVisibility::Public,
        });
        assert!(matches!(result, Err(PublishError::InvalidMemo(_))));
    }

    #[test]
    fn rejects_private_memo() {
        let result = memo_url(&MemoPublication {
            id: "memo-1".to_owned(),
            content: "content".to_owned(),
            visibility: PublicationVisibility::Private,
        });
        assert!(matches!(result, Err(PublishError::InvalidMemo(_))));
    }
}
