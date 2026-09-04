use std::path::Path;

use serde::Serialize;
use sysinfo::Disks;

use crate::CommandResponse;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Capacity {
    used_percent: f64,
    used_bytes: u64,
    total_bytes: u64,
    sampled_at: i64,
}

pub(crate) fn capacity(disks: &Disks, sampled_at: i64) -> Option<Capacity> {
    // APFS system and data volumes share capacity. Count the startup filesystem once.
    #[cfg(unix)]
    let mount = Path::new("/");
    #[cfg(windows)]
    let root = format!("{}\\", std::env::var("SystemDrive").ok()?);
    #[cfg(windows)]
    let mount = Path::new(&root);
    let disk = disks.iter().find(|disk| disk.mount_point() == mount)?;
    let total_bytes = disk.total_space();
    if total_bytes == 0 {
        return None;
    }
    let used_bytes = total_bytes.checked_sub(disk.available_space())?;
    Some(Capacity {
        used_percent: used_bytes as f64 / total_bytes as f64 * 100.0,
        used_bytes,
        total_bytes,
        sampled_at,
    })
}

#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum Category {
    System,
    Applications,
    Documents,
    Development,
    Media,
    AppData,
    Other,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Usage {
    category: Category,
    bytes: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Breakdown {
    categories: Vec<Usage>,
    incomplete: bool,
    unclassified_bytes: Option<u64>,
    sampled_at: i64,
}

#[cfg(unix)]
mod scan {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;
    use std::sync::LazyLock;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tokio::sync::Mutex;

    static CACHE: LazyLock<Mutex<Option<(Instant, Breakdown)>>> =
        LazyLock::new(|| Mutex::new(None));

    fn classify(path: &Path, directory: bool, inherited: Category) -> Category {
        if matches!(inherited, Category::Development | Category::Applications) {
            return inherited;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return inherited;
        };
        if directory {
            if matches!(
                name,
                "node_modules"
                    | ".cargo"
                    | ".rustup"
                    | ".npm"
                    | ".pnpm-store"
                    | ".gradle"
                    | ".m2"
                    | "Developer"
                    | ".git"
            ) {
                return Category::Development;
            }
            if matches!(
                name,
                "target" | "build" | "dist" | ".next" | ".nuxt" | ".venv" | "venv" | "__pycache__"
            ) && path.parent().is_some_and(|parent| {
                [
                    "Cargo.toml",
                    "package.json",
                    "pyproject.toml",
                    "CMakeLists.txt",
                ]
                .iter()
                .any(|manifest| parent.join(manifest).is_file())
            }) {
                return Category::Development;
            }
            if matches!(
                name,
                "Caches" | "Logs" | "Application Support" | "Containers" | "Group Containers"
            ) {
                return Category::AppData;
            }
            if inherited == Category::System {
                return inherited;
            }
            if name.ends_with(".app") || name == "Applications" {
                return Category::Applications;
            }
            return match name {
                "Documents" | "Desktop" | "Downloads" => Category::Documents,
                "Pictures" | "Music" | "Movies" => Category::Media,
                "Library" | ".cache" | ".config" | ".local" => Category::AppData,
                _ => inherited,
            };
        }
        if matches!(inherited, Category::System | Category::AppData) {
            return inherited;
        }
        match path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some(
                "jpg" | "jpeg" | "png" | "heic" | "avif" | "webp" | "gif" | "mp4" | "mov" | "mkv"
                | "mp3" | "m4a" | "flac" | "wav",
            ) => Category::Media,
            Some(
                "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "pages" | "numbers"
                | "key" | "txt" | "md",
            ) => Category::Documents,
            _ => inherited,
        }
    }

    fn walk(roots: Vec<(PathBuf, Category)>, deadline: Instant) -> Result<Breakdown, String> {
        let mut categories: Vec<_> = [
            Category::System,
            Category::Applications,
            Category::Documents,
            Category::Development,
            Category::Media,
            Category::AppData,
            Category::Other,
        ]
        .into_iter()
        .map(|category| Usage { category, bytes: 0 })
        .collect();
        let mut incomplete = false;
        let mut walkers: Vec<_> = roots
            .into_iter()
            .filter_map(|(path, category)| match fs::symlink_metadata(&path) {
                Ok(metadata) if !metadata.file_type().is_symlink() => Some((
                    walkdir::WalkDir::new(path)
                        .follow_links(false)
                        .follow_root_links(false)
                        .same_file_system(true)
                        .max_open(8)
                        .into_iter(),
                    vec![category],
                )),
                Ok(_) => None,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => {
                    incomplete = true;
                    None
                }
            })
            .collect();
        let mut seen = HashSet::new();
        while !walkers.is_empty() {
            if Instant::now() >= deadline || seen.len() >= 1_000_000 {
                incomplete = true;
                break;
            }
            // Interleave roots so a large system folder cannot consume the entire scan budget.
            walkers.retain_mut(|(walker, parents)| {
                let entry = match walker.next() {
                    None => return false,
                    Some(Ok(entry)) => entry,
                    Some(Err(_)) => {
                        incomplete = true;
                        return true;
                    }
                };
                if entry.file_type().is_symlink() {
                    return true;
                }
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(_) => {
                        incomplete = true;
                        if entry.file_type().is_dir() {
                            walker.skip_current_dir();
                        }
                        return true;
                    }
                };
                parents.truncate(entry.depth() + 1);
                let category = classify(entry.path(), metadata.is_dir(), parents[entry.depth()]);
                if metadata.is_dir() {
                    parents.push(category);
                }
                if seen.insert((metadata.dev(), metadata.ino())) {
                    let usage = categories
                        .iter_mut()
                        .find(|usage| usage.category == category)
                        .expect("every storage category has an accumulator");
                    usage.bytes = usage
                        .bytes
                        .saturating_add(metadata.blocks().saturating_mul(512));
                }
                true
            });
        }
        let sampled_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Could not read storage scan time: {error}"))?
            .as_secs() as i64;
        Ok(Breakdown {
            categories,
            incomplete,
            unclassified_bytes: None,
            sampled_at,
        })
    }

    pub(super) async fn read(home: PathBuf, refresh: bool) -> Result<Breakdown, String> {
        let mut cache = CACHE.lock().await;
        if let Some((time, report)) = cache.as_ref()
            && !refresh
            && time.elapsed() < Duration::from_secs(300)
        {
            return Ok(report.clone());
        }
        let result = tokio::task::spawn_blocking(move || {
            let mut roots = vec![
                (PathBuf::from("/usr"), Category::System),
                (PathBuf::from("/bin"), Category::System),
                (PathBuf::from("/sbin"), Category::System),
                (PathBuf::from("/opt"), Category::Development),
                (home, Category::Other),
            ];
            #[cfg(target_os = "macos")]
            roots.extend([
                (PathBuf::from("/System/Library"), Category::System),
                (PathBuf::from("/Library"), Category::System),
                (PathBuf::from("/private"), Category::System),
                (PathBuf::from("/Applications"), Category::Applications),
            ]);
            #[cfg(not(target_os = "macos"))]
            roots.extend([
                (PathBuf::from("/etc"), Category::System),
                (PathBuf::from("/var"), Category::System),
            ]);
            let mut devices = HashSet::from([fs::metadata("/")
                .map_err(|error| format!("Could not inspect system volume: {error}"))?
                .dev()]);
            #[cfg(target_os = "macos")]
            devices.insert(
                fs::metadata("/System/Volumes/Data")
                    .map_err(|error| format!("Could not inspect data volume: {error}"))?
                    .dev(),
            );
            #[cfg(not(target_os = "macos"))]
            let _ = &mut devices;
            roots.retain(|(path, _)| match fs::symlink_metadata(path) {
                Ok(metadata) => devices.contains(&metadata.dev()),
                // The walker distinguishes absent roots from unreadable ones.
                Err(_) => true,
            });
            let mut report = walk(roots, Instant::now() + Duration::from_secs(20))?;
            let capacity = capacity(&Disks::new_with_refreshed_list(), report.sampled_at);
            report.unclassified_bytes = capacity.and_then(|capacity| {
                capacity
                    .used_bytes
                    .checked_sub(report.categories.iter().map(|item| item.bytes).sum())
            });
            Ok::<_, String>(report)
        })
        .await
        .map_err(|error| format!("Storage scan failed: {error}"))?;
        let report = result?;
        *cache = Some((Instant::now(), report.clone()));
        Ok(report)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn classifies_development_before_documents() {
            assert_eq!(
                classify(
                    Path::new("/home/Documents/project/node_modules"),
                    true,
                    Category::Documents
                ),
                Category::Development
            );
            assert_eq!(
                classify(
                    Path::new("/home/Documents/project/node_modules/readme.md"),
                    false,
                    Category::Development
                ),
                Category::Development
            );
            assert_eq!(
                classify(
                    Path::new("/home/Documents/photo.HEIC"),
                    false,
                    Category::Documents
                ),
                Category::Media
            );
            assert_eq!(
                classify(
                    Path::new("/System/Library/file.txt"),
                    false,
                    Category::System
                ),
                Category::System
            );
        }

        #[test]
        fn classifies_nested_build_output_using_project_manifest() {
            let directory =
                std::env::temp_dir().join(format!("vesper-storage-project-{}", std::process::id()));
            let project = directory.join("Documents/project");
            let target = project.join("target");
            fs::create_dir_all(&target).unwrap();
            assert_eq!(
                classify(&target, true, Category::Documents),
                Category::Documents
            );
            fs::write(project.join("Cargo.toml"), b"[workspace]").unwrap();
            assert_eq!(
                classify(&target, true, Category::Documents),
                Category::Development
            );
            fs::write(target.join("output"), vec![1; 8192]).unwrap();
            fs::write(directory.join("Documents/report.pdf"), vec![1; 4096]).unwrap();
            let report = walk(
                vec![(directory.clone(), Category::Other)],
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            assert!(!report.incomplete);
            assert!(
                report
                    .categories
                    .iter()
                    .find(|item| item.category == Category::Development)
                    .unwrap()
                    .bytes
                    >= 8192
            );
            assert!(
                report
                    .categories
                    .iter()
                    .find(|item| item.category == Category::Documents)
                    .unwrap()
                    .bytes
                    >= 4096
            );
            fs::remove_dir_all(directory).unwrap();
        }

        #[test]
        fn scan_deduplicates_links_and_reports_deadline() {
            let directory =
                std::env::temp_dir().join(format!("vesper-storage-{}", std::process::id()));
            fs::create_dir_all(&directory).unwrap();
            let file = directory.join("document.pdf");
            fs::write(&file, vec![1; 8192]).unwrap();
            fs::hard_link(&file, directory.join("copy.pdf")).unwrap();
            std::os::unix::fs::symlink(&directory, directory.join("loop")).unwrap();
            let roots = vec![(directory.clone(), Category::Other)];
            let report = walk(roots.clone(), Instant::now() + Duration::from_secs(5)).unwrap();
            assert!(!report.incomplete);
            assert_eq!(
                report
                    .categories
                    .iter()
                    .find(|item| item.category == Category::Documents)
                    .unwrap()
                    .bytes,
                fs::metadata(&file).unwrap().blocks() * 512
            );
            assert!(walk(roots, Instant::now()).unwrap().incomplete);
            fs::remove_dir_all(directory).unwrap();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn uses_startup_capacity_once() {
        let disks = Disks::new_with_refreshed_list();
        let startup = disks
            .iter()
            .find(|disk| disk.mount_point() == Path::new("/"))
            .expect("startup filesystem");
        let sample = capacity(&disks, 1).expect("startup capacity");
        assert_eq!(sample.total_bytes, startup.total_space());
        assert_eq!(
            sample.used_bytes,
            startup.total_space() - startup.available_space()
        );
    }
}

#[tauri::command]
pub(crate) async fn read_storage(
    app: tauri::AppHandle,
    refresh: bool,
) -> CommandResponse<Breakdown> {
    #[cfg(unix)]
    {
        use tauri::Manager;
        let home = match app.path().home_dir() {
            Ok(home) => home,
            Err(error) => {
                return CommandResponse::Failed {
                    message: error.to_string(),
                };
            }
        };
        match scan::read(home, refresh).await {
            Ok(data) => CommandResponse::Ready { data },
            Err(message) => CommandResponse::Failed { message },
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (app, refresh);
        CommandResponse::Failed {
            message: "Storage categories are not available on this platform.".to_owned(),
        }
    }
}
