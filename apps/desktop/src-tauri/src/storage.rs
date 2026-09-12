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
pub(crate) async fn open_storage_settings(app: tauri::AppHandle) -> CommandResponse<()> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        use tauri_plugin_opener::OpenerExt;
        #[cfg(target_os = "macos")]
        let url = "x-apple.systempreferences:com.apple.settings.Storage";
        #[cfg(target_os = "windows")]
        let url = "ms-settings:storagesense";
        match app.opener().open_url(url, None::<String>) {
            Ok(()) => CommandResponse::Ready { data: () },
            Err(error) => CommandResponse::Failed {
                message: format!("Could not open system storage settings: {error}"),
            },
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
        CommandResponse::Failed {
            message: "Open your system disk utility to view storage details.".to_owned(),
        }
    }
}
