use crate::r2::{BUCKET, PUBLISH_PREFIX, Store, StoreError};
use futures_util::stream::{self, StreamExt, TryStreamExt};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

const UPLOAD_CONCURRENCY: usize = 8;

#[derive(Debug, PartialEq, Eq)]
pub struct PublishReport {
    pub source: PathBuf,
    pub bucket: &'static str,
    pub prefix: &'static str,
    pub objects: usize,
    pub live: bool,
}

#[derive(Debug)]
pub enum PublishError {
    MissingBuild(PathBuf),
    PathOutsideBuild {
        path: PathBuf,
        root: PathBuf,
        source: std::path::StripPrefixError,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Store(StoreError),
}

impl Display for PublishError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBuild(path) => {
                write!(
                    formatter,
                    "compiled output does not exist: {}",
                    path.display()
                )
            }
            Self::PathOutsideBuild { path, root, .. } => write!(
                formatter,
                "path {} is outside build root {}",
                path.display(),
                root.display()
            ),
            Self::Io { path, source } => {
                write!(formatter, "could not inspect {}: {source}", path.display())
            }
            Self::Store(source) => Display::fmt(source, formatter),
        }
    }
}

impl Error for PublishError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Store(source) => Some(source),
            Self::PathOutsideBuild { source, .. } => Some(source),
            Self::MissingBuild(..) => None,
        }
    }
}

impl From<StoreError> for PublishError {
    fn from(source: StoreError) -> Self {
        Self::Store(source)
    }
}

pub async fn publish(source: &Path, live: bool) -> Result<PublishReport, PublishError> {
    if !source.is_dir() {
        return Err(PublishError::MissingBuild(source.to_owned()));
    }
    let files = collect_files(source, source)?;
    let objects = files.len();
    if live {
        let store = Store::from_credentials().await?;
        stream::iter(files)
            .map(|(path, relative)| {
                let store = store.clone();
                async move {
                    let key = format!("{PUBLISH_PREFIX}/{relative}");
                    store.put_file(&key, &path).await
                }
            })
            .buffer_unordered(UPLOAD_CONCURRENCY)
            .try_collect::<Vec<()>>()
            .await?;
    }

    Ok(PublishReport {
        source: source.to_owned(),
        bucket: BUCKET,
        prefix: PUBLISH_PREFIX,
        objects,
        live,
    })
}

fn collect_files(root: &Path, directory: &Path) -> Result<Vec<(PathBuf, String)>, PublishError> {
    let entries = fs::read_dir(directory).map_err(|source| PublishError::Io {
        path: directory.to_owned(),
        source,
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| PublishError::Io {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| PublishError::Io {
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            files.extend(collect_files(root, &path)?);
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|source| PublishError::PathOutsideBuild {
                    path: path.clone(),
                    root: root.to_owned(),
                    source,
                })?
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<String>>()
                .join("/");
            files.push((path, relative));
        }
    }
    Ok(files)
}

#[cfg(test)]
#[path = "../tests/unit/publish.rs"]
mod tests;
