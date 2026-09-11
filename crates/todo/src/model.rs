use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub(crate) const MAX_TEXT_LENGTH: usize = 120;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub text: String,
    pub description: Option<String>,
    pub completed: bool,
    pub details: Option<Details>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub calendar: String,
    pub start_date: String,
    pub start_time: Option<String>,
    pub end_date: Option<String>,
    pub end_time: Option<String>,
    pub location: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub sync_error: Option<String>,
    pub date: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("calendar synchronization lock failed: {0}")]
    CalendarLock(std::io::Error),
    #[error("Notion calendar: {0}")]
    Notion(String),
    #[error(transparent)]
    Credentials(#[from] vesper_credentials::CredentialError),
    #[error("the operating-system application data directory is unavailable")]
    DataDirectoryUnavailable,
    #[error("could not determine the local date: {0}")]
    LocalDate(#[from] time::error::IndeterminateOffset),
    #[error("invalid Todo date {0}; expected YYYY-MM-DD")]
    InvalidDate(String),
    #[error("todo text cannot be empty")]
    EmptyText,
    #[error("todo text cannot exceed {MAX_TEXT_LENGTH} characters")]
    TextTooLong,
    #[error("todo item no longer exists")]
    MissingItem,
    #[error("edit imported tasks in their source calendar")]
    ImportedItem,
    #[error("todo description cannot exceed 4000 characters")]
    DescriptionTooLong,
    #[error("check-in identifier is invalid")]
    InvalidCheckIn,
    #[error("cannot check in for a future date")]
    FutureCheckIn,
    #[error("could not parse Todo schedule {path}: {message}")]
    ScheduleParse { path: PathBuf, message: String },
    #[error("Todo schedule source must be an .ics file: {0}")]
    InvalidScheduleSource(PathBuf),
    #[error("multiple Todo schedule sources use the same file name: {0}")]
    DuplicateScheduleName(String),
    #[error(transparent)]
    Database(#[from] vesper_database::Error),
    #[error("Todo database operation failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("Todo database contains an invalid item")]
    InvalidRecord,
    #[error("could not {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not calculate the next local date")]
    DateOverflow,
    #[error("Todo storage task failed: {0}")]
    Task(String),
}
