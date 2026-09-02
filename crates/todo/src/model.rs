use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use thiserror::Error;

pub(crate) const MAX_TEXT_LENGTH: usize = 120;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub text: String,
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
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub date: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Calendar {
    pub(crate) days: BTreeMap<String, Vec<Item>>,
    pub(crate) imported_occurrences: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Error)]
pub enum Error {
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
    #[error("could not parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not parse Todo schedule {path}: {message}")]
    ScheduleParse { path: PathBuf, message: String },
    #[error("Todo schedule source must be an .ics file: {0}")]
    InvalidScheduleSource(PathBuf),
    #[error("multiple Todo schedule sources use the same file name: {0}")]
    DuplicateScheduleName(String),
    #[error("could not encode the Todo list: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("could not {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Todo storage path has no parent directory")]
    MissingParent,
    #[error("could not calculate the next local date")]
    DateOverflow,
    #[error("Todo storage task failed: {0}")]
    Task(String),
}
