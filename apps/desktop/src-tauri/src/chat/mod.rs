mod projection;
mod rpc;

use crate::CommandResponse;
use projection::ChatSnapshot;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, oneshot};

#[derive(Default)]
pub(crate) struct Runtime {
    runtime: Arc<Mutex<RuntimeState>>,
    launch_lock: tokio::sync::Mutex<()>,
}

#[derive(Default)]
struct RuntimeState {
    generation: u64,
    snapshot: ChatSnapshot,
    commands: Option<mpsc::Sender<RpcRequest>>,
    process_task: Option<tauri::async_runtime::JoinHandle<()>>,
}

struct RpcRequest {
    command: serde_json::Value,
    response: oneshot::Sender<Result<(), String>>,
}

impl Runtime {
    pub(crate) fn suspend(&self, app: &tauri::AppHandle) {
        let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
        runtime.generation += 1;
        runtime.commands = None;
        if let Some(process_task) = runtime.process_task.take() {
            process_task.abort();
        }
        runtime.snapshot = ChatSnapshot {
            revision: runtime.snapshot.revision + 1,
            ..ChatSnapshot::default()
        };
        let _ = app.emit_to("main", "chat-updated", &runtime.snapshot);
    }

    fn read_snapshot(&self) -> ChatSnapshot {
        self.runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot
            .clone()
    }

    async fn send_command(&self, command: serde_json::Value) -> Result<(), String> {
        let commands = self
            .runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .commands
            .clone()
            .ok_or("Connect Pi before sending a message.")?;
        let (reply, response) = oneshot::channel();
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            commands
                .send(RpcRequest {
                    command,
                    response: reply,
                })
                .await
                .map_err(|_| "Pi disconnected.".to_owned())?;
            response.await.map_err(|_| "Pi disconnected.".to_owned())?
        })
        .await
        .map_err(|_| "Pi did not respond in time.".to_owned())?
    }

    async fn connect(
        &self,
        app: &tauri::AppHandle,
        window: &tauri::WebviewWindow,
    ) -> Result<(), String> {
        let _launch = self.launch_lock.lock().await;
        authorize(app, window)?;
        let generation = self
            .runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .generation;
        let directory = app
            .path()
            .home_dir()
            .map_err(|_| "The home directory is unavailable.")?;
        let mut snapshot = self.read_snapshot();
        if snapshot.connected {
            return Err("Pi is already connected.".into());
        }
        let child = rpc::start_process(&directory).await?;
        let (commands, receiver) = mpsc::channel(16);
        {
            let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            authorize(app, window)?;
            if runtime.generation != generation {
                return Err("Pi startup was cancelled.".into());
            }
            if let Some(process_task) = runtime.process_task.take() {
                process_task.abort();
            }
            runtime.generation += 1;
            let generation = runtime.generation;
            snapshot.connected = false;
            snapshot.busy = false;
            snapshot.error = None;
            snapshot.messages.clear();
            runtime.commands = Some(commands);
            runtime.process_task = Some(tauri::async_runtime::spawn({
                let runtime = self.runtime.clone();
                let app = app.clone();
                rpc::run_session(child, receiver, snapshot, move |snapshot| {
                    publish_snapshot(&runtime, &app, generation, snapshot)
                })
            }));
        }
        self.send_command(serde_json::json!({"type":"get_state"}))
            .await
    }
}

fn publish_snapshot(
    runtime: &Mutex<RuntimeState>,
    app: &tauri::AppHandle,
    generation: u64,
    snapshot: &mut ChatSnapshot,
) -> bool {
    let mut runtime = runtime.lock().unwrap_or_else(|e| e.into_inner());
    if runtime.generation != generation {
        return false;
    }
    snapshot.revision = runtime.snapshot.revision + 1;
    runtime.snapshot = snapshot.clone();
    let _ = app.emit_to("main", "chat-updated", &runtime.snapshot);
    true
}

fn authorize(app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" || app.state::<crate::app_lock::AppLock>().locked() {
        return Err("Unlock the main Vesper window to use Chat.".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn read_chat(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Runtime>,
) -> CommandResponse<ChatSnapshot> {
    match authorize(&app, &window) {
        Ok(()) => CommandResponse::Ready {
            data: state.read_snapshot(),
        },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) async fn connect_chat(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<ChatSnapshot> {
    let state = app.state::<Runtime>();
    match state.connect(&app, &window).await {
        Ok(()) => CommandResponse::Ready {
            data: state.read_snapshot(),
        },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) async fn send_chat(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    message: String,
) -> CommandResponse<()> {
    if let Err(message) = authorize(&app, &window) {
        return CommandResponse::Failed { message };
    }
    let state = app.state::<Runtime>();
    if message.trim().is_empty() || message.len() > 1_000_000 {
        return CommandResponse::Failed {
            message: "Enter a message shorter than 1 MB.".into(),
        };
    }
    match state
        .send_command(serde_json::json!({"type":"prompt", "message":message}))
        .await
    {
        Ok(()) => CommandResponse::Ready { data: () },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ChatAction {
    Stop,
    NewSession,
    Disconnect,
}

#[tauri::command]
pub(crate) async fn control_chat(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    action: ChatAction,
) -> CommandResponse<()> {
    if let Err(message) = authorize(&app, &window) {
        return CommandResponse::Failed { message };
    }
    let state = app.state::<Runtime>();
    if matches!(action, ChatAction::Disconnect) {
        state.suspend(&app);
        return CommandResponse::Ready { data: () };
    }
    let command = match action {
        ChatAction::Stop => "abort",
        ChatAction::NewSession => "new_session",
        ChatAction::Disconnect => unreachable!("Disconnect handled above"),
    };
    match state
        .send_command(serde_json::json!({"type":command}))
        .await
    {
        Ok(()) => CommandResponse::Ready { data: () },
        Err(message) => CommandResponse::Failed { message },
    }
}
