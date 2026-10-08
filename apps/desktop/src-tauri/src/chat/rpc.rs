use super::{RpcRequest, projection::ChatSnapshot};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, process::Stdio, time::Duration};
use tokio::{
    process::{Child, Command},
    sync::mpsc,
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

const MAX_RECORD: usize = 16 * 1024 * 1024;

#[derive(Deserialize)]
struct Record {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Response {
    id: String,
    success: bool,
    data: Option<Value>,
}

#[derive(Deserialize)]
struct PiState {
    model: Option<Model>,
}

#[derive(Deserialize)]
struct Model {
    provider: String,
    id: String,
}

#[derive(Deserialize)]
struct NewSession {
    cancelled: bool,
}

#[derive(Deserialize)]
struct PromptResult {
    disposition: Option<String>,
}

#[derive(Deserialize)]
struct ExtensionRequest {
    id: String,
    method: String,
}

pub(super) async fn start_process(directory: &std::path::Path) -> Result<Child, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new(shell)
            .args(["-lc", "printf '%s\\n' \"$PATH\"; command -v pi"])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "Resolving Pi timed out.")?
    .map_err(|_| "Could not open the login shell to find Pi.")?;
    if !output.status.success() {
        return Err(
            "Pi was not found. Install it system-wide and make it available in your login shell."
                .into(),
        );
    }
    let output =
        String::from_utf8(output.stdout).map_err(|_| "Pi returned an invalid executable path.")?;
    let mut lines = output.lines().rev();
    let executable = lines
        .next()
        .ok_or("Pi was not found in your login shell.")?;
    let path = lines
        .next()
        .ok_or("The login shell did not provide PATH.")?;
    if !std::path::Path::new(executable).is_absolute() {
        return Err("The resolved Pi executable must be an absolute path.".into());
    }
    let mut command = Command::new(executable);
    command
        .args(["--mode", "rpc", "--no-session", "--no-approve"])
        .env("PATH", path)
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command.spawn().map_err(|_| {
        "Could not start the system Pi executable. Check Pi and Node installation.".into()
    })
}

pub(super) async fn run_session(
    mut child: Child,
    mut requests: mpsc::Receiver<RpcRequest>,
    mut snapshot: ChatSnapshot,
    publish: impl Fn(&mut ChatSnapshot) -> bool,
) {
    let stdin = child.stdin.take().expect("Pi stdin is piped");
    let stdout = child.stdout.take().expect("Pi stdout is piped");
    let mut commands = FramedWrite::new(stdin, LinesCodec::new_with_max_length(MAX_RECORD));
    let mut events = FramedRead::new(stdout, LinesCodec::new_with_max_length(MAX_RECORD));
    let mut pending: HashMap<String, (RpcRequest, tokio::time::Instant)> = HashMap::new();
    let mut deadline = tokio::time::interval(Duration::from_secs(1));
    publish(&mut snapshot);
    let failure = loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(mut request) = request else {
                    break "Pi disconnected.";
                };
                let kind = request.command["type"].as_str().unwrap_or_default();
                if (kind == "prompt" || kind == "new_session") && snapshot.busy {
                    let _ = request.response.send(Err("Stop the current response before starting another.".into()));
                    continue;
                }
                if kind == "prompt" {
                    snapshot.busy = true;
                    snapshot.error = None;
                    publish(&mut snapshot);
                }
                let id = uuid::Uuid::new_v4().to_string();
                request.command["id"] = json!(id);
                let command = request.command.to_string();
                if !tokio::time::timeout(Duration::from_secs(5), commands.send(command)).await.is_ok_and(|result| result.is_ok()) {
                    let _ = request.response.send(Err("Could not send a command to Pi.".into()));
                    break "Pi stopped accepting commands.";
                }
                pending.insert(id, (request, tokio::time::Instant::now()));
            }
            event = events.next() => {
                let record = match event {
                    Some(Ok(record)) => record,
                    Some(Err(_)) => break "Pi returned an invalid or oversized RPC record.",
                    None => break "Pi exited. Reconnect to start a new chat.",
                };
                let value: Value = match serde_json::from_str(&record) {
                    Ok(value) => value,
                    Err(_) => break "Pi returned an invalid RPC record. Check the installed Pi version.",
                };
                let record = match Record::deserialize(&value) {
                    Ok(record) => record,
                    Err(_) => break "Pi returned an invalid RPC record. Check the installed Pi version.",
                };
                match record.kind.as_str() {
                    "response" => {
                        let response = match Response::deserialize(&value) {
                            Ok(response) => response,
                            Err(_) => break "Pi returned an invalid RPC response.",
                        };
                        let Some((request, _)) = pending.remove(&response.id) else {
                            continue;
                        };
                        let kind = request.command["type"].as_str().unwrap_or_default();
                        if !response.success {
                            if kind == "prompt" {
                                snapshot.busy = false;
                            }
                            let error = format!("Pi rejected {kind}. Check its configuration and installed version.");
                            snapshot.error = Some(error.clone());
                            publish(&mut snapshot);
                            let _ = request.response.send(Err(error));
                            continue;
                        }
                        match kind {
                            "get_state" => {
                                let state = response.data.and_then(|data| serde_json::from_value::<PiState>(data).ok());
                                let Some(state) = state else {
                                    let _ = request.response.send(Err("Pi returned an invalid state response.".into()));
                                    break "Pi returned an invalid state response.";
                                };
                                snapshot.connected = true;
                                snapshot.model = match state.model {
                                    Some(model) => format!("{} / {}", model.provider, model.id),
                                    None => "No model selected".into(),
                                };
                            }
                            "new_session" => {
                                let session = response.data.and_then(|data| serde_json::from_value::<NewSession>(data).ok());
                                let Some(session) = session else {
                                    let _ = request.response.send(Err("Pi returned an invalid new-session response.".into()));
                                    break "Pi returned an invalid new-session response.";
                                };
                                if session.cancelled {
                                    let _ = request.response.send(Err("An extension cancelled the new session.".into()));
                                    continue;
                                }
                                snapshot.messages.clear();
                                snapshot.error = None;
                            }
                            "prompt" => {
                                let disposition = response
                                    .data
                                    .and_then(|data| serde_json::from_value::<PromptResult>(data).ok())
                                    .and_then(|result| result.disposition);
                                if disposition.as_deref() == Some("handled") {
                                    snapshot.busy = false;
                                }
                            }
                            "abort" => snapshot.busy = false,
                            _ => {}
                        }
                        publish(&mut snapshot);
                        let _ = request.response.send(Ok(()));
                    }
                    "extension_ui_request" => {
                        let request = match ExtensionRequest::deserialize(&value) {
                            Ok(request) => request,
                            Err(_) => break "Pi returned an invalid extension dialog.",
                        };
                        if matches!(request.method.as_str(), "select" | "confirm" | "input" | "editor") {
                            let response = json!({"type":"extension_ui_response","id":request.id,"cancelled":true});
                            let result = tokio::time::timeout(Duration::from_secs(5), commands.send(response.to_string())).await;
                            if !result.is_ok_and(|result| result.is_ok()) {
                                break "Could not answer a Pi extension dialog.";
                            }
                            snapshot.error = Some("This Pi extension requested a dialog that Chat does not yet support; the dialog was cancelled.".into());
                            publish(&mut snapshot);
                        }
                    }
                    _ => {
                        if snapshot.apply(&value).is_err() {
                            break "Pi returned an invalid chat event.";
                        }
                        if !publish(&mut snapshot) {
                            break "Pi disconnected.";
                        }
                    }
                }
            }
            _ = deadline.tick() => {
                if pending.values().any(|(_, started)| started.elapsed() > Duration::from_secs(15)) {
                    break "Pi did not respond in time. Reconnect to continue.";
                }
            }
        }
    };
    let _ = child.kill().await;
    let _ = child.wait().await;
    snapshot.connected = false;
    snapshot.busy = false;
    snapshot.error = Some(failure.into());
    publish(&mut snapshot);
    for (_, (request, _)) in pending {
        let _ = request.response.send(Err(failure.into()));
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    fn fake_pi(script: &str) -> Child {
        Command::new("/bin/sh")
            .args(["-c", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap()
    }

    #[tokio::test]
    async fn correlates_unicode_responses() {
        let child = fake_pi(
            r#"
            IFS= read -r first
            first=${first#*'"id":"'}; first=${first%%'"'*}
            IFS= read -r second
            second=${second#*'"id":"'}; second=${second%%'"'*}
            printf '{"id":"%s","type":"response","success":true,"data":{"cancelled":false}}\n{"type":"message_start","message":{"role":"user","content":"hello world"}}\n' "$second"
            printf '{"id":"%s","type":"response","success":true,"data":{"model":{"provider":"test","id":"test"}}}\n' "$first"
        "#,
        );
        let (sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        let task = tokio::spawn(run_session(
            child,
            receiver,
            ChatSnapshot::default(),
            move |snapshot| events.send(snapshot.clone()).is_ok(),
        ));
        let (first, first_response) = oneshot::channel();
        let (second, second_response) = oneshot::channel();
        sender
            .send(RpcRequest {
                command: json!({"type":"get_state"}),
                response: first,
            })
            .await
            .unwrap();
        sender
            .send(RpcRequest {
                command: json!({"type":"new_session"}),
                response: second,
            })
            .await
            .unwrap();
        assert!(second_response.await.unwrap().is_ok());
        assert!(first_response.await.unwrap().is_ok());
        task.await.unwrap();
        let mut saw_messages = false;
        let mut saw_model = false;
        while let Some(snapshot) = snapshots.recv().await {
            if !snapshot.messages.is_empty() {
                saw_messages = matches!(&snapshot.messages[0].parts[0], super::super::projection::MessagePart::Text {text,..} if text == "hello\u{2028}world");
            }
            saw_model |= snapshot.model == "test / test";
        }
        assert!(saw_messages && saw_model);
    }

    #[tokio::test]
    async fn rejects_overlapping_prompts() {
        let child = fake_pi(
            r#"
            while IFS= read -r line; do
                id=${line#*'"id":"'}; id=${id%%'"'*}
                case "$line" in
                    *'"type":"prompt"'*)
                        printf '{"id":"%s","type":"response","success":true}\n{"type":"agent_start"}\n{"type":"agent_end"}\n' "$id";;
                    *'"type":"abort"'*)
                        printf '{"id":"%s","type":"response","success":true}\n{"type":"agent_settled"}\n' "$id";;
                esac
            done
        "#,
        );
        let (sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        let task = tokio::spawn(run_session(
            child,
            receiver,
            ChatSnapshot::default(),
            move |snapshot| events.send(snapshot.clone()).is_ok(),
        ));
        for (index, kind) in ["prompt", "prompt", "abort"].into_iter().enumerate() {
            let (reply, response) = oneshot::channel();
            sender
                .send(RpcRequest {
                    command: json!({"type":kind}),
                    response: reply,
                })
                .await
                .unwrap();
            let result = response.await.unwrap();
            if index == 1 {
                assert!(result.unwrap_err().contains("Stop"));
            } else {
                assert!(result.is_ok());
            }
        }
        let mut saw_busy = false;
        tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(snapshot) = snapshots.recv().await {
                saw_busy |= snapshot.busy;
                if saw_busy && !snapshot.busy {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(saw_busy);
        drop(sender);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn settles_abort() {
        let child = fake_pi(
            r#"
            while IFS= read -r line; do
                id=${line#*'"id":"'}; id=${id%%'"'*}
                case "$line" in
                    *'"type":"prompt"'*) printf '{"id":"%s","type":"response","success":true,"data":{"disposition":"started"}}\n' "$id";;
                    *'"type":"abort"'*) printf '{"id":"%s","type":"response","success":true}\n' "$id";;
                esac
            done
        "#,
        );
        let (sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        let task = tokio::spawn(run_session(
            child,
            receiver,
            ChatSnapshot::default(),
            move |snapshot| events.send(snapshot.clone()).is_ok(),
        ));
        for kind in ["prompt", "abort"] {
            let (reply, response) = oneshot::channel();
            sender
                .send(RpcRequest {
                    command: json!({"type":kind}),
                    response: reply,
                })
                .await
                .unwrap();
            response.await.unwrap().unwrap();
        }
        let mut saw_busy = false;
        let mut settled = false;
        tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(snapshot) = snapshots.recv().await {
                saw_busy |= snapshot.busy;
                if saw_busy && !snapshot.busy {
                    settled = true;
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(settled);
        drop(sender);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn settles_handled_prompt() {
        let child = fake_pi(
            r#"
            while IFS= read -r line; do
                id=${line#*'"id":"'}; id=${id%%'"'*}
                case "$line" in
                    *'"type":"prompt"'*) printf '{"id":"%s","type":"response","success":true,"data":{"disposition":"handled"}}\n' "$id";;
                esac
            done
        "#,
        );
        let (sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        let task = tokio::spawn(run_session(
            child,
            receiver,
            ChatSnapshot::default(),
            move |snapshot| events.send(snapshot.clone()).is_ok(),
        ));
        let (reply, response) = oneshot::channel();
        sender
            .send(RpcRequest {
                command: json!({"type":"prompt"}),
                response: reply,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), response)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let settled = tokio::time::timeout(Duration::from_secs(3), async {
            let mut saw_busy = false;
            while let Some(snapshot) = snapshots.recv().await {
                saw_busy |= snapshot.busy;
                if saw_busy && !snapshot.busy {
                    return true;
                }
            }
            false
        })
        .await
        .unwrap_or(false);
        drop(sender);
        task.await.unwrap();
        assert!(settled);
    }

    #[tokio::test]
    async fn disconnects_on_protocol_error() {
        let child = fake_pi("read -r line; printf 'not json\\n'");
        let (sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        let task = tokio::spawn(run_session(
            child,
            receiver,
            ChatSnapshot::default(),
            move |snapshot| events.send(snapshot.clone()).is_ok(),
        ));
        let (reply, response) = oneshot::channel();
        sender
            .send(RpcRequest {
                command: json!({"type":"get_state"}),
                response: reply,
            })
            .await
            .unwrap();
        assert!(response.await.unwrap().unwrap_err().contains("invalid RPC"));
        task.await.unwrap();
        let mut last = ChatSnapshot::default();
        while let Some(snapshot) = snapshots.recv().await {
            last = snapshot;
        }
        assert!(!last.connected && !last.busy);
        assert!(last.error.unwrap().contains("invalid RPC"));
    }
    #[tokio::test]
    async fn handles_command_outcomes() {
        for (kind, success, data, error, remaining) in [
            ("get_state", true, json!({"model":null}), None, 1),
            (
                "get_state",
                true,
                json!({"model":{"id":"missing-provider"}}),
                Some("invalid state"),
                1,
            ),
            ("new_session", true, json!({"cancelled":false}), None, 0),
            (
                "new_session",
                true,
                json!({"cancelled":true}),
                Some("cancelled"),
                1,
            ),
            (
                "new_session",
                true,
                json!({}),
                Some("invalid new-session"),
                1,
            ),
            ("prompt", false, json!(null), Some("Pi rejected prompt"), 1),
            ("prompt", true, json!({"disposition":"handled"}), None, 1),
        ] {
            let record = json!({"id":"%s","type":"response","success":success,"data":data,"error":"private-provider-error"});
            let script = format!(
                r#"
                read -r line
                id=${{line#*'"id":"'}}; id=${{id%%'"'*}}
                printf '%s\n' '{{"type":"message_start","message":{{"role":"user","content":"keep"}}}}'
                printf '{record}\n' "$id"
            "#
            );
            let (sender, receiver) = mpsc::channel(16);
            let (events, mut snapshots) = mpsc::unbounded_channel();
            let task = tokio::spawn(run_session(
                fake_pi(&script),
                receiver,
                ChatSnapshot::default(),
                move |snapshot| events.send(snapshot.clone()).is_ok(),
            ));
            let (reply, response) = oneshot::channel();
            sender
                .send(RpcRequest {
                    command: json!({"type":kind}),
                    response: reply,
                })
                .await
                .unwrap();
            let result = response.await.unwrap();
            if let Some(error) = error {
                let message = result.unwrap_err();
                assert!(message.contains(error), "{message}");
                assert!(!message.contains("private-provider-error"));
            } else {
                assert!(result.is_ok());
            }
            task.await.unwrap();
            let mut last = ChatSnapshot::default();
            while let Some(snapshot) = snapshots.recv().await {
                last = snapshot;
            }
            assert_eq!(last.messages.len(), remaining);
            assert!(!last.busy);
            if kind == "get_state" && error.is_none() {
                assert_eq!(last.model, "No model selected");
            }
        }
    }

    #[tokio::test]
    async fn cancels_extension_dialogs() {
        let child = fake_pi(
            r#"
            printf '%s\n' '{"type":"extension_ui_request","id":"dialog","method":"confirm"}'
            read -r line
            case "$line" in
                *'"cancelled":true'*)
                    case "$line" in
                        *'"id":"dialog"'*) printf '%s\n' '{"type":"message_start","message":{"role":"user","content":"dialog cancelled"}}';;
                    esac;;
            esac
        "#,
        );
        let (_sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        run_session(child, receiver, ChatSnapshot::default(), move |snapshot| {
            events.send(snapshot.clone()).is_ok()
        })
        .await;
        let mut cancelled = false;
        let mut explained = false;
        while let Some(snapshot) = snapshots.recv().await {
            cancelled |= snapshot.messages.iter().any(|message| matches!(&message.parts[0], super::super::projection::MessagePart::Text { text, .. } if text == "dialog cancelled"));
            explained |= snapshot
                .error
                .is_some_and(|error| error.contains("dialog was cancelled"));
        }
        assert!(cancelled && explained);
    }

    #[tokio::test]
    async fn disconnects_unresponsive_pi() {
        let child = fake_pi("read -r line; read -r next");
        let (sender, receiver) = mpsc::channel(16);
        let (events, mut snapshots) = mpsc::unbounded_channel();
        let task = tokio::spawn(run_session(
            child,
            receiver,
            ChatSnapshot::default(),
            move |snapshot| events.send(snapshot.clone()).is_ok(),
        ));
        let (reply, response) = oneshot::channel();
        sender
            .send(RpcRequest {
                command: json!({"type":"get_state"}),
                response: reply,
            })
            .await
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(20), response)
            .await
            .unwrap()
            .unwrap();
        assert!(result.unwrap_err().contains("did not respond in time"));
        task.await.unwrap();
        let mut last = ChatSnapshot::default();
        while let Some(snapshot) = snapshots.recv().await {
            last = snapshot;
        }
        assert!(!last.connected && !last.busy);
        assert!(last.error.unwrap().contains("did not respond in time"));
    }
}

#[cfg(all(test, unix))]
mod installed_pi_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a system Pi installation"]
    async fn starts_ephemeral_system_pi() {
        let home = std::env::var("HOME").unwrap();
        let mut child = start_process(std::path::Path::new(&home)).await.unwrap();
        let mut commands = FramedWrite::new(child.stdin.take().unwrap(), LinesCodec::new());
        let mut events = FramedRead::new(child.stdout.take().unwrap(), LinesCodec::new());
        commands
            .send(json!({"id":"probe","type":"get_state"}).to_string())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = events.next().await {
                let response: Value = serde_json::from_str(&event.unwrap()).unwrap();
                if response["id"] == "probe" {
                    assert_eq!(response["success"], true);
                    assert!(response["data"]["sessionFile"].is_null());
                    assert_eq!(response["data"]["messageCount"], 0);
                    return;
                }
            }
            panic!("Pi exited before responding");
        })
        .await
        .unwrap();
        drop(commands);
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap();
    }
}
