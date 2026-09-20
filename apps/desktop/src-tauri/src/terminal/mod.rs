mod devices;
mod session;

use crate::CommandResponse;
use devices::Snapshot;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri::{Emitter, Manager, ipc::Channel};

#[derive(Default)]
pub(crate) struct Runtime {
    active: AtomicBool,
    generation: AtomicU64,
    lifecycle: Mutex<()>,
    launches: Mutex<HashMap<String, String>>,
    refresh: tokio::sync::Mutex<()>,
    snapshot: Mutex<Snapshot>,
    sessions: session::Sessions,
}
impl Runtime {
    pub(crate) fn suspend(&self) {
        let _gate = self
            .lifecycle
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.active.store(false, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.sessions.close_all();
    }
    fn claim_launch(&self, device_id: &str, session_id: &str) -> Result<(), String> {
        let mut launches = self
            .launches
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if launches.get(device_id).map(String::as_str) != Some(session_id) {
            return Err("A newer connection replaced this terminal request.".into());
        }
        launches.remove(device_id);
        Ok(())
    }
    fn check_generation(&self, generation: u64) -> Result<(), String> {
        if generation != self.generation.load(Ordering::SeqCst)
            || !self.active.load(Ordering::SeqCst)
        {
            return Err("Terminal access changed. Reopen the terminal.".into());
        }
        Ok(())
    }
    async fn discover(&self, app: &tauri::AppHandle) -> Snapshot {
        let _refresh = self.refresh.lock().await;
        if !self.active.load(Ordering::SeqCst) {
            return self
                .snapshot
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
        }
        let generation = self.generation.load(Ordering::SeqCst);
        let result = devices::discover().await;
        let _gate = self
            .lifecycle
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if generation != self.generation.load(Ordering::SeqCst) {
            return snapshot.clone();
        }
        match result {
            Ok(next) => {
                if !snapshot.local_id.is_empty() && next.local_id != snapshot.local_id {
                    self.sessions.close_remote();
                }
                *snapshot = next;
            }
            Err(error) => {
                for device in &mut snapshot.devices {
                    device.online = None;
                }
                snapshot.error = Some(error);
            }
        }
        let result = snapshot.clone();
        let _ = app.emit_to("main", "ssh-devices-changed", &result);
        result
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.sessions.close_all();
    }
}

fn authorize(app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("The terminal is only available in the main application window.".into());
    }
    if app.state::<crate::configuration::AppLockState>().locked() {
        return Err("Unlock Vesper to use the terminal.".into());
    }
    Ok(())
}

pub(crate) fn start_monitoring(app: tauri::AppHandle) {
    let idle_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            idle_app
                .state::<Runtime>()
                .sessions
                .expire_idle(std::time::Instant::now());
        }
    });
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let runtime = app.state::<Runtime>();
            if runtime.active.load(Ordering::SeqCst) {
                runtime.discover(&app).await;
            }
        }
    });
}

#[tauri::command]
pub(crate) fn set_terminal_active(
    active: bool,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    if window.label() != "main" {
        return CommandResponse::Failed {
            message: "The terminal is only available in the main application window.".into(),
        };
    }
    let runtime = app.state::<Runtime>();
    if !active {
        runtime.suspend();
        return CommandResponse::Ready { data: () };
    }
    let _gate = runtime
        .lifecycle
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Err(message) = authorize(&app, &window) {
        return CommandResponse::Failed { message };
    }
    runtime.active.store(true, Ordering::SeqCst);
    CommandResponse::Ready { data: () }
}

#[tauri::command]
pub(crate) async fn read_ssh_devices(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<Snapshot> {
    if let Err(message) = authorize(&app, &window) {
        return CommandResponse::Failed { message };
    }
    let data = app.state::<Runtime>().discover(&app).await;
    CommandResponse::Ready { data }
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Target {
    Local,
    Ssh {
        #[serde(rename = "deviceId")]
        device_id: String,
        username: String,
    },
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConnectRequest {
    session_id: String,
    target: Target,
    cols: u16,
    rows: u16,
}

#[tauri::command]
pub(crate) async fn connect_terminal(
    request: ConnectRequest,
    output: Channel<session::Output>,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    if let Err(message) = authorize(&app, &window) {
        return CommandResponse::Failed { message };
    }
    let runtime = app.state::<Runtime>();
    let generation = runtime.generation.load(Ordering::SeqCst);
    let target_key = match &request.target {
        Target::Local => "local".to_owned(),
        Target::Ssh { device_id, .. } => format!("ssh:{device_id}"),
    };
    runtime
        .launches
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(target_key.clone(), request.session_id.clone());
    let result = tauri::async_runtime::spawn_blocking(move || {
        let runtime = app.state::<Runtime>();
        let _gate = runtime
            .lifecycle
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        runtime.claim_launch(&target_key, &request.session_id)?;
        authorize(&app, &window)?;
        runtime.check_generation(generation)?;
        let dimensions = session::validate_size(request.cols, request.rows)?;
        match request.target {
            Target::Local => runtime
                .sessions
                .connect_local(request.session_id, dimensions, output),
            Target::Ssh {
                device_id,
                username,
            } => {
                let snapshot = runtime
                    .snapshot
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                if snapshot.error.is_some() {
                    return Err("Refresh Tailscale successfully before connecting.".into());
                }
                let device = snapshot
                    .devices
                    .iter()
                    .find(|device| device.id == device_id)
                    .cloned()
                    .ok_or("This device is no longer in your tailnet. Refresh the device list.")?;
                drop(snapshot);
                runtime
                    .sessions
                    .connect(request.session_id, &device, &username, dimensions, output)
            }
        }
    })
    .await;
    match result {
        Ok(Ok(())) => CommandResponse::Ready { data: () },
        Ok(Err(message)) => CommandResponse::Failed { message },
        Err(_) => CommandResponse::Failed {
            message: "The terminal launcher stopped unexpectedly.".into(),
        },
    }
}

#[tauri::command]
pub(crate) async fn write_terminal(
    session_id: String,
    bytes: Vec<u8>,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        authorize(&app, &window)?;
        app.state::<Runtime>().sessions.write(&session_id, bytes)
    })
    .await;
    match result {
        Ok(Ok(())) => CommandResponse::Ready { data: () },
        Ok(Err(message)) => CommandResponse::Failed { message },
        Err(_) => CommandResponse::Failed {
            message: "Could not send terminal input.".into(),
        },
    }
}

#[tauri::command]
pub(crate) fn record_terminal_activity(
    session_id: String,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    let result = authorize(&app, &window)
        .and_then(|()| app.state::<Runtime>().sessions.record_activity(&session_id));
    match result {
        Ok(()) => CommandResponse::Ready { data: () },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) fn resize_terminal(
    session_id: String,
    cols: u16,
    rows: u16,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    let result = authorize(&app, &window)
        .and_then(|()| session::validate_size(cols, rows))
        .and_then(|dimensions| {
            app.state::<Runtime>()
                .sessions
                .resize(&session_id, dimensions)
        });
    match result {
        Ok(()) => CommandResponse::Ready { data: () },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) fn acknowledge_terminal(
    session_id: String,
    bytes: usize,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    let result = authorize(&app, &window).and_then(|()| {
        app.state::<Runtime>()
            .sessions
            .acknowledge(&session_id, bytes)
    });
    match result {
        Ok(()) => CommandResponse::Ready { data: () },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) fn disconnect_terminal(
    session_id: String,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<()> {
    if window.label() != "main" {
        return CommandResponse::Failed {
            message: "The terminal is only available in the main application window.".into(),
        };
    }
    let runtime = app.state::<Runtime>();
    let _gate = runtime
        .lifecycle
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    runtime.sessions.close(&session_id);
    CommandResponse::Ready { data: () }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_latest_device_launch_can_start() {
        let runtime = Runtime::default();
        runtime
            .launches
            .lock()
            .unwrap()
            .insert("node".into(), "old".into());
        runtime
            .launches
            .lock()
            .unwrap()
            .insert("node".into(), "new".into());
        assert!(runtime.claim_launch("node", "old").is_err());
        assert!(runtime.claim_launch("node", "new").is_ok());
        assert!(runtime.claim_launch("node", "old").is_err());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn rejects_stale_connections() {
        let runtime = Runtime::default();
        runtime.active.store(true, Ordering::SeqCst);
        let queued_generation = runtime.generation.load(Ordering::SeqCst);
        assert!(runtime.check_generation(queued_generation).is_ok());
        runtime.suspend();
        runtime.active.store(true, Ordering::SeqCst);
        let _gate = runtime.lifecycle.lock().unwrap();
        assert!(runtime.check_generation(queued_generation).is_err());
        assert!(
            runtime
                .check_generation(runtime.generation.load(Ordering::SeqCst))
                .is_ok()
        );
    }
}
