use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdout, Command};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsage {
    pub plan_type: Option<String>,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
    pub spark: Option<CodexLimit>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLimit {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindow {
    pub used_percent: f64,
    pub window_duration_mins: Option<u64>,
    pub resets_at: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsResult {
    rate_limits: RateLimits,
    #[serde(default)]
    rate_limits_by_limit_id: HashMap<String, CodexLimit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimits {
    plan_type: Option<String>,
    primary: Option<RateLimitWindow>,
    secondary: Option<RateLimitWindow>,
}

pub async fn read() -> Result<CodexUsage, String> {
    let binary = resolve_codex_binary()?;
    let mut child = Command::new(&binary)
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Could not start Codex CLI at {}: {error}", binary.display()))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Codex app-server stdin is unavailable".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Codex app-server stdout is unavailable".to_owned())?;
    let mut stdout = BufReader::new(stdout);
    let stderr = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        let mut output = String::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_string(&mut output).await;
        }
        output
    });

    write_message(
        &mut stdin,
        &json!({
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "vesper",
                    "title": "Vesper",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": { "experimentalApi": true }
            }
        }),
    )
    .await?;
    read_result(&mut stdout, 1).await?;

    write_message(&mut stdin, &json!({ "method": "initialized" })).await?;
    write_message(
        &mut stdin,
        &json!({ "id": 2, "method": "account/rateLimits/read" }),
    )
    .await?;
    let result = read_result(&mut stdout, 2).await;

    drop(stdin);
    let _ = child.kill().await;
    let _ = child.wait().await;
    let stderr = stderr_task.await.unwrap_or_default();
    let value = result.map_err(|error| {
        let stderr = stderr.trim();
        if stderr.is_empty() {
            error
        } else {
            format!("{error}: {stderr}")
        }
    })?;
    parse_usage(value)
}

async fn write_message(
    stdin: &mut tokio::process::ChildStdin,
    message: &Value,
) -> Result<(), String> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|error| format!("Could not encode Codex request: {error}"))?;
    encoded.push(b'\n');
    stdin
        .write_all(&encoded)
        .await
        .map_err(|error| format!("Could not write to Codex app-server: {error}"))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("Could not flush Codex request: {error}"))
}

async fn read_result(
    stdout: &mut BufReader<ChildStdout>,
    expected_id: i64,
) -> Result<Value, String> {
    tokio::time::timeout(RESPONSE_TIMEOUT, async {
        loop {
            let mut line = String::new();
            let read = stdout
                .read_line(&mut line)
                .await
                .map_err(|error| format!("Could not read Codex response: {error}"))?;
            if read == 0 {
                return Err("Codex app-server closed before returning usage".to_owned());
            }
            let message: Value = match serde_json::from_str(&line) {
                Ok(message) => message,
                Err(_) => continue,
            };
            if message.get("id").and_then(Value::as_i64) != Some(expected_id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(format!("Codex app-server returned an error: {error}"));
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| "Codex response did not include a result".to_owned());
        }
    })
    .await
    .map_err(|error| format!("Timed out while reading Codex usage: {error}"))?
}

fn parse_usage(value: Value) -> Result<CodexUsage, String> {
    let response: RateLimitsResult = serde_json::from_value(value)
        .map_err(|error| format!("Codex returned an unsupported usage payload: {error}"))?;
    let mut spark = None;
    for (id, limit) in response.rate_limits_by_limit_id {
        let id_is_spark = id.to_lowercase().contains("spark");
        let limit_id_is_spark = match limit.limit_id.as_deref() {
            Some(limit_id) => limit_id.to_lowercase().contains("spark"),
            None => false,
        };
        let limit_name_is_spark = match limit.limit_name.as_deref() {
            Some(limit_name) => limit_name.to_lowercase().contains("spark"),
            None => false,
        };
        if id_is_spark || limit_id_is_spark || limit_name_is_spark {
            spark = Some(limit);
            break;
        }
    }
    Ok(CodexUsage {
        plan_type: response.rate_limits.plan_type,
        primary: response.rate_limits.primary,
        secondary: response.rate_limits.secondary,
        spark,
    })
}

fn resolve_codex_binary() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CODEX_BINARY").map(PathBuf::from)
        && path.is_file()
    {
        return Ok(path);
    }

    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|directory| directory.join(if cfg!(windows) { "codex.exe" } else { "codex" }))
            .find(|candidate| candidate.is_file())
    }) {
        return Ok(path);
    }

    #[cfg(unix)]
    {
        let shell = std::env::var("SHELL").unwrap_or("/bin/zsh".to_owned());
        if let Ok(output) = std::process::Command::new(shell)
            .args(["-lc", "command -v codex"])
            .output()
        {
            let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
            if output.status.success() && path.is_file() {
                return Ok(path);
            }
        }
    }

    Err("Codex CLI was not found. Install it and run `codex login` first.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rate_limit_windows() {
        let usage = parse_usage(json!({
            "rateLimits": {
                "planType": "plus",
                "primary": { "usedPercent": 12, "windowDurationMins": 300, "resetsAt": 1_800_000_000 },
                "secondary": { "usedPercent": 34, "windowDurationMins": 10_080, "resetsAt": 1_800_100_000 }
            },
            "rateLimitsByLimitId": {
                "codex_spark": {
                    "limitId": "gpt-5.3-codex-spark",
                    "limitName": "Codex Spark",
                    "primary": { "usedPercent": 8, "windowDurationMins": 300, "resetsAt": 1_800_000_000 },
                    "secondary": { "usedPercent": 19, "windowDurationMins": 10_080, "resetsAt": 1_800_100_000 }
                }
            }
        }))
        .expect("valid usage response");

        assert_eq!(usage.plan_type.as_deref(), Some("plus"));
        assert_eq!(usage.primary.expect("primary").used_percent, 12.0);
        assert_eq!(
            usage.secondary.expect("secondary").window_duration_mins,
            Some(10_080)
        );
        let spark = usage.spark.expect("spark limit");
        assert_eq!(spark.limit_id.as_deref(), Some("gpt-5.3-codex-spark"));
        assert_eq!(spark.primary.expect("spark primary").used_percent, 8.0);
    }

    #[tokio::test]
    #[ignore = "requires a locally authenticated Codex CLI"]
    async fn reads_usage_from_local_codex() {
        let usage = read().await.expect("Codex usage should be readable");
        assert!(usage.primary.is_some() || usage.secondary.is_some());
    }
}
