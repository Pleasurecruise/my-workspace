use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const SOURCE_DIRECTORY: &str = "content";
static NEXT_BUILD: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
pub struct BuildReport {
    pub markdown_files: usize,
    pub copied_files: usize,
}

#[derive(Debug)]
pub struct BuildOutput {
    directory: PathBuf,
    report: BuildReport,
}

impl BuildOutput {
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub const fn report(&self) -> &BuildReport {
        &self.report
    }
}

impl Drop for BuildOutput {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.directory) {
            eprintln!(
                "could not remove temporary build directory {}: {error}",
                self.directory.display()
            );
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompiledContent {
    pub version: u8,
    pub documents: Vec<CompiledDocument>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompiledDocument {
    pub path: String,
    pub html: String,
}

#[derive(Debug)]
pub enum BuildError {
    MissingSource(PathBuf),
    UnsupportedSymlink(PathBuf),
    OutputCollision(PathBuf),
    PathOutsideRoot {
        path: PathBuf,
        root: PathBuf,
        source: std::path::StripPrefixError,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize(serde_json::Error),
}

impl Display for BuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSource(path) => {
                write!(
                    formatter,
                    "content source does not exist: {}",
                    path.display()
                )
            }
            Self::UnsupportedSymlink(path) => {
                write!(
                    formatter,
                    "content source contains a symlink: {}",
                    path.display()
                )
            }
            Self::OutputCollision(path) => write!(
                formatter,
                "more than one source file maps to output: {}",
                path.display()
            ),
            Self::PathOutsideRoot { path, root, .. } => write!(
                formatter,
                "path {} is outside expected root {}",
                path.display(),
                root.display()
            ),
            Self::Io { path, source } => {
                write!(formatter, "could not process {}: {source}", path.display())
            }
            Self::Serialize(source) => {
                write!(formatter, "could not serialize content index: {source}")
            }
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Serialize(source) => Some(source),
            Self::PathOutsideRoot { source, .. } => Some(source),
            Self::MissingSource(..) | Self::UnsupportedSymlink(..) | Self::OutputCollision(..) => {
                None
            }
        }
    }
}

pub fn build(repository: &Path) -> Result<BuildOutput, BuildError> {
    let source = repository.join(SOURCE_DIRECTORY);
    if !source.is_dir() {
        return Err(BuildError::MissingSource(source));
    }

    let sequence = NEXT_BUILD.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("vesper-publish-{}-{sequence}", std::process::id()));
    if directory.exists() {
        io(&directory, fs::remove_dir_all(&directory))?;
    }
    io(&directory, fs::create_dir_all(&directory))?;

    let mut output = BuildOutput {
        directory,
        report: BuildReport {
            markdown_files: 0,
            copied_files: 0,
        },
    };
    let mut outputs = HashSet::new();
    outputs.insert(output.directory.join("content.json"));
    let mut documents = Vec::new();
    compile_directory(
        &source,
        &source,
        &output.directory,
        &mut outputs,
        &mut documents,
        &mut output.report,
    )?;

    let content = CompiledContent {
        version: 1,
        documents,
    };
    let index = serde_json::to_vec_pretty(&content).map_err(BuildError::Serialize)?;
    let index_path = output.directory.join("content.json");
    io(&index_path, fs::write(&index_path, index))?;
    Ok(output)
}

fn compile_directory(
    root: &Path,
    directory: &Path,
    staging: &Path,
    outputs: &mut HashSet<PathBuf>,
    documents: &mut Vec<CompiledDocument>,
    report: &mut BuildReport,
) -> Result<(), BuildError> {
    let mut entries: Vec<fs::DirEntry> = io(directory, fs::read_dir(directory))?
        .collect::<Result<Vec<fs::DirEntry>, std::io::Error>>()
        .map_err(|source| BuildError::Io {
            path: directory.to_owned(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = io(&path, entry.file_type())?;
        if file_type.is_symlink() {
            return Err(BuildError::UnsupportedSymlink(path));
        }
        if file_type.is_dir() {
            compile_directory(root, &path, staging, outputs, documents, report)?;
            continue;
        }
        if !file_type.is_file() || is_ignored(&path) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|source| BuildError::PathOutsideRoot {
                path: path.clone(),
                root: root.to_owned(),
                source,
            })?;
        let mut destination = staging.join(relative);
        let markdown = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
        if markdown {
            destination.set_extension("html");
        }
        if !outputs.insert(destination.clone()) {
            return Err(BuildError::OutputCollision(destination));
        }

        if let Some(parent) = destination.parent() {
            io(parent, fs::create_dir_all(parent))?;
        }
        if markdown {
            let source = io(&path, fs::read_to_string(&path))?;
            let html = crate::markdown::render(&source);
            io(&destination, fs::write(&destination, &html))?;
            let relative_output = destination.strip_prefix(staging).map_err(|source| {
                BuildError::PathOutsideRoot {
                    path: destination.clone(),
                    root: staging.to_owned(),
                    source,
                }
            })?;
            documents.push(CompiledDocument {
                path: web_path(relative_output),
                html,
            });
            report.markdown_files += 1;
        } else {
            io(&destination, fs::copy(&path, &destination))?;
            report.copied_files += 1;
        }
    }
    Ok(())
}

fn web_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/")
}

fn is_ignored(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".DS_Store") | Some("Thumbs.db")
    )
}

fn io<T>(path: &Path, result: std::io::Result<T>) -> Result<T, BuildError> {
    result.map_err(|source| BuildError::Io {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
#[path = "../tests/unit/build.rs"]
mod tests;
