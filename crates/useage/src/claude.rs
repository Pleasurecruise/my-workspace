use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA: &str = "oauth-2025-04-20";
#[cfg(target_os = "macos")]
const KEYCHAIN_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsage {
    pub plan_type: String,
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub used_percent: f64,
    pub window_duration_mins: u64,
    pub resets_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialsFile {
    claude_ai_oauth: Option<OAuthCredentials>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthCredentials {
    access_token: Option<String>,
    subscription_type: Option<String>,
    expires_at: Option<u64>,
}

struct Credentials {
    access_token: String,
    subscription_type: String,
}

#[derive(Deserialize)]
struct UsageResponse {
    five_hour: Option<UsageResponseWindow>,
    seven_day: Option<UsageResponseWindow>,
}

#[derive(Deserialize)]
struct UsageResponseWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

pub async fn read() -> Result<ClaudeUsage, String> {
    let credentials = read_credentials().await?;
    let plan_type = plan_name(&credentials.subscription_type)
        .ok_or_else(|| "Claude Code is not signed in with a Claude subscription".to_owned())?;
    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| format!("Could not create Claude usage client: {error}"))?
        .get(USAGE_URL)
        .bearer_auth(credentials.access_token)
        .header("anthropic-beta", OAUTH_BETA)
        .header(
            reqwest::header::USER_AGENT,
            concat!("vesper/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|error| format!("Could not query Claude usage: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Claude usage request failed: HTTP {status}"));
    }
    let response: UsageResponse = response
        .json()
        .await
        .map_err(|error| format!("Claude returned an unsupported usage payload: {error}"))?;
    parse_usage(plan_type, response)
}

fn parse_usage(plan_type: String, response: UsageResponse) -> Result<ClaudeUsage, String> {
    let usage = ClaudeUsage {
        plan_type,
        five_hour: usage_window(response.five_hour, 5 * 60),
        seven_day: usage_window(response.seven_day, 7 * 24 * 60),
    };
    if usage.five_hour.is_none() && usage.seven_day.is_none() {
        return Err("Claude usage response did not contain a supported quota window".to_owned());
    }
    Ok(usage)
}

fn usage_window(window: Option<UsageResponseWindow>, duration_mins: u64) -> Option<UsageWindow> {
    let window = window?;
    let utilization = window.utilization.filter(|value| value.is_finite())?;
    Some(UsageWindow {
        used_percent: utilization.clamp(0.0, 100.0),
        window_duration_mins: duration_mins,
        resets_at: window.resets_at.filter(|value| !value.trim().is_empty()),
    })
}

async fn read_credentials() -> Result<Credentials, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Could not locate the home directory for Claude Code".to_owned())?;
    let credentials_path = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"))
        .join(".credentials.json");
    let file_credentials = read_credentials_file(&credentials_path).await;

    #[cfg(target_os = "macos")]
    if let Some(mut credentials) = read_keychain_credentials().await {
        if credentials.subscription_type.is_empty()
            && let Ok(file_credentials) = &file_credentials
        {
            credentials.subscription_type = file_credentials.subscription_type.clone();
        }
        return Ok(credentials);
    }

    file_credentials
}

async fn read_credentials_file(path: &Path) -> Result<Credentials, String> {
    let content = tokio::fs::read_to_string(path).await.map_err(|error| {
        format!(
            "Could not read Claude Code credentials at {}: {error}",
            path.display()
        )
    })?;
    parse_credentials(&content)
}

#[cfg(target_os = "macos")]
async fn read_keychain_credentials() -> Option<Credentials> {
    let mut command = tokio::process::Command::new("/usr/bin/security");
    command
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let output = match tokio::time::timeout(KEYCHAIN_TIMEOUT, command.output()).await {
        Ok(Ok(output)) if output.status.success() => output,
        Ok(Ok(_)) => return None,
        Ok(Err(error)) => {
            tracing::debug!(%error, "could not read Claude Code credentials from Keychain");
            return None;
        }
        Err(_) => {
            tracing::debug!("timed out while reading Claude Code credentials from Keychain");
            return None;
        }
    };
    let content = String::from_utf8(output.stdout).ok()?;
    parse_credentials(content.trim()).ok()
}

fn parse_credentials(content: &str) -> Result<Credentials, String> {
    let file: CredentialsFile = serde_json::from_str(content)
        .map_err(|error| format!("Could not decode Claude Code OAuth credentials: {error}"))?;
    let oauth = file
        .claude_ai_oauth
        .ok_or_else(|| "Claude Code does not contain an OAuth session".to_owned())?;
    if oauth
        .expires_at
        .is_some_and(|expires_at| expires_at <= unix_time_millis())
    {
        return Err(
            "Claude Code OAuth session has expired; run `claude auth login` again".to_owned(),
        );
    }
    let access_token = oauth
        .access_token
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| "Claude Code OAuth session does not contain an access token".to_owned())?;
    Ok(Credentials {
        access_token,
        subscription_type: oauth.subscription_type.unwrap_or_default(),
    })
}

fn unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn plan_name(subscription_type: &str) -> Option<String> {
    let normalized = subscription_type.trim();
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("max") {
        Some("Max".to_owned())
    } else if lower.contains("pro") {
        Some("Pro".to_owned())
    } else if lower.contains("team") {
        Some("Team".to_owned())
    } else if normalized.is_empty() || lower.contains("api") {
        None
    } else {
        let mut characters = normalized.chars();
        let first = characters.next()?;
        Some(first.to_uppercase().chain(characters).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_clamps_usage_windows() {
        let response: UsageResponse = serde_json::from_value(serde_json::json!({
            "five_hour": { "utilization": 12.5, "resets_at": "2026-09-01T12:00:00Z" },
            "seven_day": { "utilization": 104, "resets_at": "2026-09-07T12:00:00Z" }
        }))
        .expect("valid usage response");

        let usage = parse_usage("Max".to_owned(), response).expect("supported usage response");
        assert_eq!(
            usage.five_hour.expect("five-hour window").used_percent,
            12.5
        );
        assert_eq!(
            usage.seven_day.expect("seven-day window").used_percent,
            100.0
        );
    }

    #[test]
    fn parses_credentials_and_plan() {
        let credentials = parse_credentials(
            r#"{"claudeAiOauth":{"accessToken":"oauth-token","subscriptionType":"max_20x","expiresAt":4102444800000}}"#,
        )
        .expect("valid credentials");

        assert_eq!(credentials.access_token, "oauth-token");
        assert_eq!(
            plan_name(&credentials.subscription_type).as_deref(),
            Some("Max")
        );
        assert_eq!(plan_name("api"), None);
    }

    #[test]
    fn rejects_payload_without_supported_windows() {
        let response: UsageResponse = serde_json::from_value(serde_json::json!({
            "five_hour": { "utilization": null }
        }))
        .expect("valid response shape");

        assert!(parse_usage("Pro".to_owned(), response).is_err());
    }

    #[tokio::test]
    #[ignore = "requires a locally authenticated Claude Code subscription"]
    async fn reads_live_usage() {
        let usage = read().await.expect("Claude usage should be readable");
        assert!(usage.five_hour.is_some() || usage.seven_day.is_some());
    }
}
