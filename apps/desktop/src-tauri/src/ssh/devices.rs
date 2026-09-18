use std::{collections::HashMap, net::IpAddr, path::PathBuf, time::Duration};
use tokio::io::AsyncReadExt;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Device {
    pub id: String,
    pub name: String,
    pub dns_name: String,
    pub address: String,
    pub os: String,
    pub online: Option<bool>,
    pub username: String,
}

#[derive(Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub devices: Vec<Device>,
    pub error: Option<String>,
    #[serde(skip)]
    pub local_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Status {
    backend_state: String,
    #[serde(rename = "Self")]
    local: Option<Peer>,
    peer: Option<HashMap<String, Peer>>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Peer {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(default)]
    host_name: String,
    #[serde(rename = "DNSName", default)]
    dns_name: String,
    #[serde(rename = "TailscaleIPs", default)]
    addresses: Vec<IpAddr>,
    #[serde(rename = "OS", default)]
    os: String,
    online: Option<bool>,
    tags: Option<Vec<String>>,
}

pub(super) fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty()
        || username.len() > 64
        || username.starts_with('-')
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("Enter the remote device's login username (letters, numbers, dot, underscore or hyphen).".into());
    }
    Ok(())
}

fn is_tailnet_address(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            octets[0] == 100 && (64..=127).contains(&octets[1])
        }
        IpAddr::V6(address) => address.segments()[..3] == [0xfd7a, 0x115c, 0xa1e0],
    }
}

fn parse(bytes: &[u8], default_username: &str) -> Result<Snapshot, String> {
    let status: Status = serde_json::from_slice(bytes)
        .map_err(|_| "Tailscale returned an unreadable device list.".to_owned())?;
    if status.backend_state != "Running" {
        return Err(
            "Tailscale is not connected. Open Tailscale and connect to your tailnet, then refresh."
                .into(),
        );
    }
    let local = status.local;
    let local_id = local
        .as_ref()
        .map(|local| local.id.clone())
        .unwrap_or_default();
    let mut devices = Vec::new();
    for (key, peer) in status.peer.unwrap_or_default() {
        let id = if peer.id.is_empty() { key } else { peer.id };
        if id == local_id {
            continue;
        }
        if local.as_ref().is_some_and(|local| {
            peer.addresses
                .iter()
                .any(|address| local.addresses.contains(address))
        }) {
            continue;
        }
        if !peer
            .tags
            .as_ref()
            .is_some_and(|tags| tags.iter().any(|tag| tag == "tag:server"))
        {
            continue;
        }
        let Some(address) = peer
            .addresses
            .iter()
            .find(|address| is_tailnet_address(address))
        else {
            continue;
        };
        let name = peer
            .dns_name
            .split('.')
            .next()
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                if peer.host_name.is_empty() {
                    address.to_string()
                } else {
                    peer.host_name
                }
            });
        devices.push(Device {
            username: default_username.to_owned(),
            id,
            name,
            dns_name: peer.dns_name.trim_end_matches('.').to_owned(),
            address: address.to_string(),
            os: peer.os,
            online: peer.online,
        });
    }
    devices.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(Snapshot {
        devices,
        error: None,
        local_id,
    })
}

fn find_tailscale_binary() -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|path| {
                    path.join(if cfg!(windows) {
                        "tailscale.exe"
                    } else {
                        "tailscale"
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    #[cfg(unix)]
    candidates.extend(
        [
            "/opt/homebrew/bin/tailscale",
            "/usr/local/bin/tailscale",
            "/usr/bin/tailscale",
            "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
        ]
        .map(PathBuf::from),
    );
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(path).join("Tailscale/tailscale.exe"));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "Tailscale CLI was not found. Install and sign in to Tailscale, then refresh.".into()
        })
}

pub(super) async fn discover() -> Result<Snapshot, String> {
    let mut command = tokio::process::Command::new(find_tailscale_binary()?);
    command
        .args(["status", "--json"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    tokio::time::timeout(Duration::from_secs(8), async {
        let mut child = command
            .spawn()
            .map_err(|_| "Could not start the Tailscale CLI.".to_owned())?;
        let output = child
            .stdout
            .take()
            .ok_or("Could not read Tailscale output.")?;
        let mut bytes = Vec::new();
        output
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "Could not read Tailscale output.".to_owned())?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err("Tailscale's device list exceeds the supported size.".into());
        }
        let exit = child
            .wait()
            .await
            .map_err(|_| "Could not finish Tailscale discovery.".to_owned())?;
        if !exit.success() {
            return Err(
                "Could not query Tailscale. Check that the app is running and signed in.".into(),
            );
        }
        let default_username =
            std::env::var(if cfg!(windows) { "USERNAME" } else { "USER" }).unwrap_or_default();
        let default_username = if validate_username(&default_username).is_ok() {
            default_username
        } else {
            String::new()
        };
        parse(&bytes, &default_username)
    })
    .await
    .map_err(|_| "Tailscale discovery timed out. Check your local Tailscale client.".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_server_peers() {
        let json = br#"{"BackendState":"Running","Self":{"ID":"self"},"Peer":{"key":{"ID":"nas","Tags":["tag:server"],"HostName":"NAS","DNSName":"nas.example.ts.net.","TailscaleIPs":["100.64.0.2"],"OS":"linux","Online":true},"off":{"ID":"off","Tags":["tag:server"],"HostName":"Server","TailscaleIPs":["fd7a:115c:a1e0::1"],"Online":false},"bad":{"ID":"bad","Tags":["tag:server"],"TailscaleIPs":["127.0.0.1"]}}}"#;
        let snapshot = parse(json, "local").unwrap();
        assert_eq!(snapshot.devices.len(), 2);
        assert_eq!(snapshot.devices[0].username, "local");
        assert_eq!(snapshot.devices[0].dns_name, "nas.example.ts.net");
        assert_eq!(snapshot.devices[1].online, Some(false));
    }
    #[test]
    fn prefers_device_alias() {
        let bytes = br#"{"BackendState":"Running","Peer":{
            "alias":{"ID":"alias","HostName":"VM-16-6-ubuntu","DNSName":"tencent.example.ts.net.","Tags":["tag:server"],"TailscaleIPs":["100.64.0.2"]},
            "host":{"ID":"host","HostName":"fallback-host","Tags":["tag:server"],"TailscaleIPs":["100.64.0.3"]},
            "address":{"ID":"address","Tags":["tag:server"],"TailscaleIPs":["100.64.0.4"]}
        }}"#;
        let snapshot = parse(bytes, "local").unwrap();
        let names: HashMap<_, _> = snapshot
            .devices
            .iter()
            .map(|device| (device.id.as_str(), device.name.as_str()))
            .collect();
        assert_eq!(names["alias"], "tencent");
        assert_eq!(names["host"], "fallback-host");
        assert_eq!(names["address"], "100.64.0.4");
        assert_eq!(
            snapshot
                .devices
                .iter()
                .find(|device| device.id == "alias")
                .unwrap()
                .dns_name,
            "tencent.example.ts.net"
        );
    }
    #[test]
    fn filters_server_tag() {
        let peers = [
            ("server", serde_json::json!(["tag:server"])),
            ("multi", serde_json::json!(["tag:other", "tag:server"])),
            ("other", serde_json::json!(["tag:other"])),
            ("similar", serde_json::json!(["tag:server-backup"])),
            ("empty", serde_json::json!([])),
            ("null", serde_json::Value::Null),
        ];
        let mut values = serde_json::Map::new();
        for (id, tags) in peers {
            values.insert(
                id.into(),
                serde_json::json!({"ID": id, "TailscaleIPs": ["100.64.0.2"], "Tags": tags}),
            );
        }
        values.insert(
            "missing".into(),
            serde_json::json!({"ID": "missing", "TailscaleIPs": ["100.64.0.3"]}),
        );
        let bytes =
            serde_json::to_vec(&serde_json::json!({"BackendState": "Running", "Peer": values}))
                .unwrap();
        let snapshot = parse(&bytes, "local").unwrap();
        let ids: Vec<_> = snapshot
            .devices
            .iter()
            .map(|device| device.id.as_str())
            .collect();
        assert_eq!(ids, ["multi", "server"]);
    }
    #[test]
    fn excludes_local_device() {
        let bytes = br#"{"BackendState":"Running","Self":{"ID":"self","TailscaleIPs":["100.64.0.1"]},"Peer":{
            "self":{"ID":"self","Tags":["tag:server"],"TailscaleIPs":["100.64.0.1"]},
            "alias":{"ID":"alias","Tags":["tag:server"],"TailscaleIPs":["100.64.0.1"]},
            "offline":{"ID":"offline","Tags":["tag:server"],"TailscaleIPs":["100.64.0.2"],"Online":false,"Active":false},
            "idle":{"ID":"idle","Tags":["tag:server"],"TailscaleIPs":["100.64.0.3"],"Online":true,"Active":false}
        }}"#;
        let snapshot = parse(bytes, "local").unwrap();
        assert_eq!(snapshot.devices.len(), 2);
        assert_eq!(snapshot.devices[0].id, "idle");
        assert_eq!(snapshot.devices[1].id, "offline");
        assert_eq!(snapshot.devices[1].online, Some(false));
    }
    #[test]
    fn reports_unavailable_status() {
        assert!(parse(br#"{"BackendState":"NeedsLogin"}"#, "local").is_err());
        assert!(parse(b"broken", "local").is_err());
        assert!(
            parse(br#"{"BackendState":"Running","Peer":null}"#, "local")
                .unwrap()
                .devices
                .is_empty()
        );
    }
    #[test]
    fn rejects_invalid_usernames() {
        for value in [
            "",
            "-oProxyCommand=bad",
            "user@host",
            "user;cmd",
            "user\nother",
        ] {
            assert!(validate_username(value).is_err());
        }
        assert!(validate_username("admin").is_ok());
    }
}
