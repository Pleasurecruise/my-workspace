use super::devices::Device;
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{Arc, Condvar, Mutex, mpsc},
    time::{Duration, Instant},
};
use tauri::ipc::Channel;

const OUTPUT_WINDOW: usize = 128 * 1024;
const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum Output {
    Data { bytes: Vec<u8> },
    Exit { code: Option<u32> },
    Error { message: String },
}

#[derive(Default)]
struct Pending {
    bytes: usize,
    stopped: bool,
}
#[derive(Default)]
struct Flow {
    pending: Mutex<Pending>,
    changed: Condvar,
}
impl Flow {
    fn reserve(&self, count: usize) -> bool {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while !pending.stopped && pending.bytes + count > OUTPUT_WINDOW {
            pending = self
                .changed
                .wait(pending)
                .unwrap_or_else(|error| error.into_inner());
        }
        if pending.stopped {
            return false;
        }
        pending.bytes += count;
        true
    }
    fn acknowledge(&self, count: usize) -> Result<(), String> {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if count > pending.bytes {
            return Err("Invalid terminal output acknowledgement.".into());
        }
        pending.bytes -= count;
        self.changed.notify_all();
        Ok(())
    }
    fn stop(&self) {
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stopped = true;
        self.changed.notify_all();
    }
}

struct Session {
    node_id: String,
    last_activity: Mutex<Instant>,
    output: Channel<Output>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>,
    input: mpsc::SyncSender<Vec<u8>>,
    flow: Arc<Flow>,
}
impl Session {
    fn stop(&self) {
        self.flow.stop();
        let _ = self
            .killer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .kill();
    }
}

#[derive(Default)]
pub(super) struct Sessions(Arc<Mutex<HashMap<String, Arc<Session>>>>);

pub(super) fn validate_size(cols: u16, rows: u16) -> Result<PtySize, String> {
    if !(2..=500).contains(&cols) || !(1..=300).contains(&rows) {
        return Err("Terminal dimensions are outside the supported range.".into());
    }
    Ok(PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    })
}

fn build_command(device: &Device, username: &str) -> Result<CommandBuilder, String> {
    super::devices::validate_username(username)?;
    // The address is taken from Tailscale discovery, never supplied as a shell command.
    let address = device
        .address
        .parse::<std::net::IpAddr>()
        .map_err(|_| "Invalid Tailscale address.".to_owned())?;
    #[cfg(windows)]
    let binary = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .ok_or("Windows system directory is unavailable.")?
        .join("System32/OpenSSH/ssh.exe");
    #[cfg(not(windows))]
    let binary = std::path::PathBuf::from("/usr/bin/ssh");
    if !binary.is_file() {
        return Err(
            "OpenSSH client was not found. Install the system OpenSSH client and reconnect.".into(),
        );
    }
    let mut command = CommandBuilder::new(binary);
    command.args([
        "-F",
        "none",
        "-tt",
        "-o",
        "ConnectTimeout=12",
        "-o",
        "ServerAliveInterval=15",
        "-o",
        "ServerAliveCountMax=3",
        "-o",
        "ForwardAgent=no",
        "-o",
        "ClearAllForwardings=yes",
        "-o",
        "PermitLocalCommand=no",
        "-o",
        "StrictHostKeyChecking=ask",
        "-p",
        "22",
        "-l",
        username,
    ]);
    command.arg(address.to_string());
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    Ok(command)
}

impl Sessions {
    pub(super) fn connect(
        &self,
        id: String,
        device: &Device,
        username: &str,
        dimensions: PtySize,
        output: Channel<Output>,
    ) -> Result<(), String> {
        self.spawn(
            id,
            &device.id,
            build_command(device, username)?,
            dimensions,
            output,
        )
    }
    fn spawn(
        &self,
        id: String,
        node_id: &str,
        command: CommandBuilder,
        dimensions: PtySize,
        output: Channel<Output>,
    ) -> Result<(), String> {
        if uuid::Uuid::parse_str(&id).is_err() {
            return Err("Invalid terminal session ID.".into());
        }
        let mut sessions = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if sessions.contains_key(&id) || sessions.values().any(|session| session.node_id == node_id)
        {
            return Err(
                "This device already has an open terminal. Disconnect it before reconnecting."
                    .into(),
            );
        }
        if sessions.len() >= 4 {
            return Err(
                "Disconnect another terminal before connecting (maximum four sessions).".into(),
            );
        }
        let pair = native_pty_system()
            .openpty(dimensions)
            .map_err(|_| "Could not allocate a native terminal.".to_owned())?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| "Could not read the native terminal.".to_owned())?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|_| "Could not write to the native terminal.".to_owned())?;
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("Could not start OpenSSH: {error}"))?;
        drop(pair.slave);
        let killer = Arc::new(Mutex::new(child.clone_killer()));
        let flow = Arc::new(Flow::default());
        let (input, receiver) = mpsc::sync_channel::<Vec<u8>>(16);
        sessions.insert(
            id.clone(),
            Arc::new(Session {
                node_id: node_id.to_owned(),
                last_activity: Mutex::new(Instant::now()),
                output: output.clone(),
                master: Mutex::new(pair.master),
                killer: killer.clone(),
                input,
                flow: flow.clone(),
            }),
        );
        drop(sessions);
        let input_output = output.clone();
        let input_flow = flow.clone();
        let input_killer = killer.clone();
        std::thread::spawn(move || {
            for bytes in receiver {
                if writer
                    .write_all(&bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    let pending = input_flow
                        .pending
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if !pending.stopped {
                        let _ = input_output.send(Output::Error {
                            message: "The SSH terminal stopped accepting input.".into(),
                        });
                    }
                    drop(pending);
                    let _ = input_killer
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .kill();
                    break;
                }
            }
        });
        let sessions = self.0.clone();
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                let count = match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => count,
                    // PTYs commonly report EIO when their slave closes.
                    Err(_) => break,
                };
                if !flow.reserve(count)
                    || output
                        .send(Output::Data {
                            bytes: buffer[..count].to_vec(),
                        })
                        .is_err()
                {
                    let _ = killer
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .kill();
                    break;
                }
            }
            let code = child.wait().ok().map(|status| status.exit_code());
            flow.stop();
            sessions
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&id);
            let _ = output.send(Output::Exit { code });
        });
        Ok(())
    }
    pub(super) fn write(&self, id: &str, bytes: Vec<u8>) -> Result<(), String> {
        if bytes.len() > 4096 {
            return Err("Terminal input chunk exceeds 4 KB.".into());
        }
        let session = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(id)
            .cloned()
            .ok_or("This SSH session is closed.")?;
        session
            .input
            .send(bytes)
            .map_err(|_| "This SSH session is closed.".into())
    }
    pub(super) fn record_activity(&self, id: &str) -> Result<(), String> {
        let sessions = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let session = sessions.get(id).ok_or("This SSH session is closed.")?;
        *session
            .last_activity
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Instant::now();
        Ok(())
    }
    pub(super) fn resize(&self, id: &str, dimensions: PtySize) -> Result<(), String> {
        let session = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(id)
            .cloned()
            .ok_or("This SSH session is closed.")?;
        session
            .master
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .resize(dimensions)
            .map_err(|_| "Could not resize the terminal.".into())
    }
    pub(super) fn acknowledge(&self, id: &str, bytes: usize) -> Result<(), String> {
        let session = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(id)
            .cloned();
        if let Some(session) = session {
            session.flow.acknowledge(bytes)?;
        }
        Ok(())
    }
    pub(super) fn expire_idle(&self, now: Instant) {
        let mut sessions = self.0.lock().unwrap_or_else(|error| error.into_inner());
        sessions.retain(|_, session| {
            let last_activity = *session
                .last_activity
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if now.saturating_duration_since(last_activity) < IDLE_TIMEOUT {
                return true;
            }
            session.stop();
            let _ = session.output.send(Output::Error {
                message: "Disconnected after 5 minutes without terminal activity. \
                    Select Reconnect to connect again."
                    .into(),
            });
            false
        });
    }
    pub(super) fn close(&self, id: &str) {
        if let Some(session) = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(id)
        {
            session.stop();
        }
    }
    pub(super) fn close_all(&self) {
        for (_, session) in self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .drain()
        {
            session.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_ssh_arguments() {
        let device = Device {
            id: "node".into(),
            name: "Server".into(),
            dns_name: String::new(),
            address: "100.64.0.2".into(),
            os: "linux".into(),
            online: Some(true),
            username: "admin".into(),
        };
        let command = build_command(&device, "admin").unwrap();
        let args = command.get_argv();
        assert_eq!(args.last().unwrap(), "100.64.0.2");
        assert!(args.iter().any(|arg| arg == "StrictHostKeyChecking=ask"));
        assert!(args.iter().any(|arg| arg == "ForwardAgent=no"));
        assert!(args.windows(2).any(|pair| pair == ["-F", "none"]));
        let configuration = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(configuration.path(), "Host *\n HostName 192.0.2.10\n ProxyCommand false\n RemoteCommand echo inherited\n ControlMaster auto\n ControlPersist 600\n ForwardX11 yes\n").unwrap();
        let result = std::process::Command::new(&args[0])
            .args(["-G", "-F"])
            .arg(configuration.path())
            .args(&args[1..])
            .output()
            .unwrap();
        assert!(result.status.success());
        let settings = String::from_utf8(result.stdout).unwrap();
        for expected in [
            "hostname 100.64.0.2",
            "controlmaster false",
            "controlpersist no",
            "forwardx11 no",
        ] {
            assert!(settings.lines().any(|line| line == expected));
        }
        assert!(!settings.lines().any(|line| line.starts_with("proxycommand ") || line.starts_with("remotecommand ")));
    }
    #[test]
    fn validates_terminal_bounds() {
        assert!(validate_size(0, 24).is_err());
        assert!(validate_size(80, 0).is_err());
        assert!(validate_size(80, 24).is_ok());
        let flow = Flow::default();
        assert!(flow.reserve(8192));
        assert!(flow.acknowledge(8193).is_err());
        flow.acknowledge(8192).unwrap();
        flow.stop();
        assert!(!flow.reserve(1));
    }
    #[test]
    fn stopping_unblocks_backpressure() {
        let flow = Arc::new(Flow::default());
        assert!(flow.reserve(OUTPUT_WINDOW));
        let reader = flow.clone();
        let thread = std::thread::spawn(move || reader.reserve(1));
        flow.stop();
        assert!(!thread.join().unwrap());
    }
    #[cfg(unix)]
    #[test]
    fn transports_terminal_io() {
        let sessions = Sessions::default();
        let id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::channel();
        let output = Channel::<Output>::new(move |body| {
            if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                let _ = sender.send(serde_json::from_str::<serde_json::Value>(&json).unwrap());
            }
            Ok(())
        });
        let mut command = CommandBuilder::new("/bin/sh");
        command.args([
            "-c",
            "read line; printf 'received:%s\\n' \"$line\"; read again",
        ]);
        sessions
            .spawn(
                id.clone(),
                "test",
                command,
                validate_size(80, 24).unwrap(),
                output,
            )
            .unwrap();
        sessions
            .resize(&id, validate_size(100, 30).unwrap())
            .unwrap();
        sessions.write(&id, b"hello-terminal\n".to_vec()).unwrap();
        let mut received = String::new();
        loop {
            let event = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            if let Some(bytes) = event["bytes"].as_array() {
                let bytes: Vec<u8> = bytes
                    .iter()
                    .map(|byte| byte.as_u64().unwrap() as u8)
                    .collect();
                sessions.acknowledge(&id, bytes.len()).unwrap();
                received.push_str(&String::from_utf8_lossy(&bytes));
                if received.contains("received:hello-terminal") {
                    break;
                }
            }
        }
        sessions.close_all();
        loop {
            let event = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            if event["kind"] == "exit" {
                break;
            }
        }
        assert!(sessions.write(&id, b"closed".to_vec()).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn expires_idle_sessions() {
        let sessions = Sessions::default();
        let id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::channel();
        let output = Channel::<Output>::new(move |body| {
            if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                let _ = sender.send(serde_json::from_str::<serde_json::Value>(&json).unwrap());
            }
            Ok(())
        });
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "printf background; while read line; do :; done"]);
        sessions
            .spawn(
                id.clone(),
                "idle-test",
                command,
                validate_size(80, 24).unwrap(),
                output,
            )
            .unwrap();
        let session = sessions.0.lock().unwrap().get(&id).cloned().unwrap();
        *session.last_activity.lock().unwrap() = Instant::now() - IDLE_TIMEOUT;
        sessions.record_activity(&id).unwrap();
        sessions.write(&id, b"input".to_vec()).unwrap();
        let last_activity = *session.last_activity.lock().unwrap();
        let event = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(event["kind"], "data");
        sessions
            .acknowledge(&id, event["bytes"].as_array().unwrap().len())
            .unwrap();
        sessions
            .resize(&id, validate_size(100, 30).unwrap())
            .unwrap();
        sessions.write(&id, b"\x1b[1;1R".to_vec()).unwrap();
        sessions.write(&id, Vec::new()).unwrap();
        assert_eq!(*session.last_activity.lock().unwrap(), last_activity);
        sessions.expire_idle(last_activity + IDLE_TIMEOUT - Duration::from_secs(1));
        assert!(sessions.0.lock().unwrap().contains_key(&id));
        sessions.expire_idle(last_activity + IDLE_TIMEOUT);
        assert!(!sessions.0.lock().unwrap().contains_key(&id));
        let mut explained = false;
        loop {
            let event = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
            if event["kind"] == "error" {
                explained = event["message"].as_str().unwrap().contains("5 minutes");
            }
            if event["kind"] == "exit" {
                break;
            }
        }
        assert!(explained);
        assert!(sessions.write(&id, b"closed".to_vec()).is_err());
    }
}
