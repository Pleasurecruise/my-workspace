use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdout;

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);
const WEEKLY_PERIOD: &str = "USAGE_PERIOD_TYPE_WEEKLY";
const WEEKLY_DURATION_MINS: u64 = 7 * 24 * 60;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrokUsage {
    pub plan_type: Option<String>,
    pub window: UsageWindow,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub used_percent: f64,
    pub window_duration_mins: Option<u64>,
    pub resets_at: Option<String>,
}

pub async fn read() -> Result<GrokUsage, String> {
    let binary = resolve_grok_binary()?;
    let mut child = tokio::process::Command::new(&binary)
        .args(["agent", "stdio"])
        .env("GROK_DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| {
            format!(
                "Could not start Grok runtime at {}: {error}",
                binary.display()
            )
        })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Grok runtime stdin is unavailable".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Grok runtime stdout is unavailable".to_owned())?;
    let mut stdout = BufReader::new(stdout);
    write_message(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": 1, "clientCapabilities": {} }
        }),
    )
    .await?;
    read_result(&mut stdout, 1).await?;
    write_message(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "_x.ai/billing", "params": {} }),
    )
    .await?;
    let result = read_result(&mut stdout, 2).await;

    drop(stdin);
    if let Err(error) = child.kill().await {
        tracing::debug!(%error, "Grok runtime already stopped");
    }
    if let Err(error) = child.wait().await {
        tracing::debug!(%error, "could not reap Grok runtime");
    }
    let value = result?;
    parse_usage(value.get("result").unwrap_or(&value))
}

async fn write_message(
    stdin: &mut tokio::process::ChildStdin,
    message: &Value,
) -> Result<(), String> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|error| format!("Could not encode Grok request: {error}"))?;
    encoded.push(b'\n');
    stdin
        .write_all(&encoded)
        .await
        .map_err(|error| format!("Could not write to Grok runtime: {error}"))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("Could not flush Grok request: {error}"))
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
                .map_err(|error| format!("Could not read Grok response: {error}"))?;
            if read == 0 {
                return Err("Grok runtime closed before returning usage".to_owned());
            }
            let message: Value = match serde_json::from_str(&line) {
                Ok(message) => message,
                Err(_) => continue,
            };
            if message.get("id").and_then(Value::as_i64) != Some(expected_id) {
                continue;
            }
            if message.get("error").is_some() {
                return Err("Grok runtime could not return billing usage".to_owned());
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| "Grok response did not include a result".to_owned());
        }
    })
    .await
    .map_err(|error| format!("Timed out while reading Grok usage: {error}"))?
}

fn parse_usage(billing: &Value) -> Result<GrokUsage, String> {
    let config = billing
        .get("config")
        .filter(|value| value.is_object())
        .ok_or_else(|| "Grok billing response did not contain a configuration".to_owned())?;
    let period = config
        .get("currentPeriod")
        .or_else(|| {
            config
                .get("history")
                .and_then(Value::as_array)
                .and_then(|rows| rows.last())
        })
        .unwrap_or(config);
    let may_borrow_current_totals = config
        .get("currentPeriod")
        .is_some_and(|current| std::ptr::eq(current, period))
        || std::ptr::eq(config, period);
    let fallback = may_borrow_current_totals.then_some(config);
    let used_percent = explicit_percent(period)
        .or_else(|| fallback.and_then(explicit_percent))
        .or_else(|| percentage_from_totals(period, fallback))
        .or_else(|| official_zero_percent(config))
        .ok_or_else(|| "Grok billing response did not contain usable quota usage".to_owned())?
        .clamp(0.0, 100.0);
    let start = string_field(period, "start")
        .or_else(|| string_field(period, "billingPeriodStart"))
        .or_else(|| string_field(config, "billingPeriodStart"));
    let end = string_field(period, "end")
        .or_else(|| string_field(period, "billingPeriodEnd"))
        .or_else(|| string_field(config, "billingPeriodEnd"));
    let window_duration_mins = if string_field(period, "type") == Some(WEEKLY_PERIOD) {
        Some(WEEKLY_DURATION_MINS)
    } else {
        measured_duration_mins(start, end)
    };
    let plan_type = billing
        .get("subscriptionTier")
        .or_else(|| billing.get("subscription_tier"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    Ok(GrokUsage {
        plan_type,
        window: UsageWindow {
            used_percent,
            window_duration_mins,
            resets_at: end.map(str::to_owned),
        },
    })
}

fn explicit_percent(value: &Value) -> Option<f64> {
    value.get("creditUsagePercent").and_then(nonnegative_number)
}

fn percentage_from_totals(period: &Value, fallback: Option<&Value>) -> Option<f64> {
    let limit = amount_field(period, &["monthlyLimit"])
        .or_else(|| fallback.and_then(|value| amount_field(value, &["monthlyLimit"])))?;
    if limit <= 0.0 {
        return None;
    }
    let used = amount_field(period, &["totalUsed", "includedUsed", "used"])
        .or_else(|| fallback.and_then(|value| amount_field(value, &["totalUsed", "used"])))?;
    Some((used / limit) * 100.0)
}

fn official_zero_percent(config: &Value) -> Option<f64> {
    let period = config.get("currentPeriod")?;
    (config.get("isUnifiedBillingUser").and_then(Value::as_bool) == Some(true)
        && string_field(period, "type") == Some(WEEKLY_PERIOD)
        && amount_field(config, &["onDemandCap"]) == Some(0.0)
        && amount_field(config, &["onDemandUsed"]) == Some(0.0)
        && amount_field(config, &["prepaidBalance"]) == Some(0.0))
    .then_some(0.0)
}

fn amount_field(value: &Value, names: &[&str]) -> Option<f64> {
    names
        .iter()
        .find_map(|name| value.get(name).and_then(amount))
}

fn amount(value: &Value) -> Option<f64> {
    nonnegative_number(value.get("val").unwrap_or(value))
}

fn nonnegative_number(value: &Value) -> Option<f64> {
    let value = value.as_f64()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn string_field<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn measured_duration_mins(start: Option<&str>, end: Option<&str>) -> Option<u64> {
    let start = OffsetDateTime::parse(start?, &Rfc3339).ok()?;
    let end = OffsetDateTime::parse(end?, &Rfc3339).ok()?;
    let minutes = (end - start).whole_minutes();
    (minutes > 0).then(|| minutes.try_into().ok()).flatten()
}

fn resolve_grok_binary() -> Result<PathBuf, String> {
    for variable in ["GROK_BINARY", "GROK_PATH"] {
        if let Some(path) = std::env::var_os(variable).map(PathBuf::from)
            && path.is_file()
        {
            return Ok(path);
        }
    }
    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|directory| directory.join(if cfg!(windows) { "grok.exe" } else { "grok" }))
            .find(|candidate| candidate.is_file())
    }) {
        return Ok(path);
    }
    Err("Grok runtime was not found. Install it, run `grok login --device-auth`, and optionally set GROK_BINARY.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_weekly_billing() {
        let usage = parse_usage(&json!({
            "config": {
                "creditUsagePercent": 42.5,
                "currentPeriod": {
                    "type": WEEKLY_PERIOD,
                    "start": "2026-06-01T00:00:00Z",
                    "end": "2026-06-08T00:00:00Z"
                }
            },
            "subscription_tier": "SuperGrok Heavy"
        }))
        .expect("valid Grok billing");

        assert_eq!(usage.plan_type.as_deref(), Some("SuperGrok Heavy"));
        assert_eq!(usage.window.used_percent, 42.5);
        assert_eq!(
            usage.window.window_duration_mins,
            Some(WEEKLY_DURATION_MINS)
        );
    }

    #[test]
    fn falls_back_to_legacy_totals() {
        let usage = parse_usage(&json!({
            "config": { "monthlyLimit": { "val": 2_000 }, "used": { "val": 500 } }
        }))
        .expect("legacy billing totals");

        assert_eq!(usage.window.used_percent, 25.0);
    }

    #[test]
    fn recognizes_official_zero_usage_shape() {
        let usage = parse_usage(&json!({
            "config": {
                "isUnifiedBillingUser": true,
                "currentPeriod": { "type": WEEKLY_PERIOD },
                "onDemandCap": { "val": 0 },
                "onDemandUsed": { "val": 0 },
                "prepaidBalance": { "val": 0 }
            }
        }))
        .expect("fresh unified billing");

        assert_eq!(usage.window.used_percent, 0.0);
    }

    #[test]
    fn does_not_pair_historical_period_with_current_totals() {
        assert!(
            parse_usage(&json!({
                "config": {
                    "history": [{ "start": "2026-05-01T00:00:00Z", "end": "2026-05-08T00:00:00Z" }],
                    "monthlyLimit": { "val": 2_000 },
                    "used": { "val": 500 }
                }
            }))
            .is_err()
        );
    }

    #[tokio::test]
    #[ignore = "requires a locally authenticated official Grok runtime"]
    async fn reads_live_usage() {
        let usage = read().await.expect("Grok usage should be readable");
        assert!((0.0..=100.0).contains(&usage.window.used_percent));
    }
}
