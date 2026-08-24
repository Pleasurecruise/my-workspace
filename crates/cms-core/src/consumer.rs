use crate::api::memos::MemoView;
use crate::r2::{Store, StoreError};
use serde::Serialize;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub use crate::api::moment::Photo as PhotoItem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Memos,
    Moment,
    Knowledge,
}

impl TryFrom<&str> for Channel {
    type Error = ConsumerError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "memos" => Ok(Self::Memos),
            "moment" => Ok(Self::Moment),
            "knowledge" => Ok(Self::Knowledge),
            invalid_channel => Err(ConsumerError::UnknownChannel(invalid_channel.to_owned())),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "channel",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChannelView {
    Memos {
        connected: bool,
        memos: Vec<MemoView>,
        tags: Vec<crate::api::memos::TagCount>,
        next_cursor: Option<String>,
    },
    Moment {
        connected: bool,
        photos: Vec<PhotoItem>,
        tags: Vec<String>,
        total: usize,
        next_cursor: Option<String>,
    },
    Knowledge {
        connected: bool,
        knowledge: Vec<crate::api::knowledge::Document>,
        next_cursor: Option<String>,
    },
}

#[derive(Debug)]
pub enum ConsumerError {
    UnknownChannel(String),
    Store(StoreError),
}

impl Display for ConsumerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownChannel(channel) => {
                write!(formatter, "unknown consumer channel: {channel}")
            }
            Self::Store(source) => Display::fmt(source, formatter),
        }
    }
}

impl Error for ConsumerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Store(source) => Some(source),
            Self::UnknownChannel(..) => None,
        }
    }
}

impl From<StoreError> for ConsumerError {
    fn from(source: StoreError) -> Self {
        Self::Store(source)
    }
}

pub struct Repository {
    store: Store,
}

impl Repository {
    pub const fn new(store: Store) -> Self {
        Self { store }
    }

    pub const fn store(&self) -> &Store {
        &self.store
    }
}

pub async fn asset(key: &str, repository: &Repository) -> Result<Vec<u8>, ConsumerError> {
    let outside_image_prefix = !key.starts_with("img/");
    let contains_reserved_character = key.contains(['\\', '%', '?', '#']);
    let contains_parent_segment = key.split('/').any(|part| matches!(part, "." | ".."));
    if outside_image_prefix || contains_reserved_character || contains_parent_segment {
        return Err(ConsumerError::Store(StoreError::Request(
            "only img/ object keys can be displayed".to_owned(),
        )));
    }
    Ok(repository.store.get(key).await?)
}

#[cfg(test)]
#[path = "../tests/unit/consumer.rs"]
mod tests;
