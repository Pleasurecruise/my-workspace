use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const QUERY_TIMEOUT: Duration = Duration::from_secs(15);
const USER_ENDPOINT: &str = "copilot_internal/user";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct CopilotUsage {
    pub login: Option<String>,
    pub copilot_plan: Option<String>,
    pub access_type_sku: Option<String>,
    pub quota_reset_date_utc: Option<String>,
    pub quota_snapshots: CopilotQuotaSnapshots,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct CopilotQuotaSnapshots {
    pub chat: Option<CopilotQuota>,
    pub completions: Option<CopilotQuota>,
    pub premium_interactions: Option<CopilotQuota>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct CopilotQuota {
    pub entitlement: Option<f64>,
    pub remaining: Option<f64>,
    pub percent_remaining: Option<f64>,
    pub unlimited: Option<bool>,
    pub overage_count: Option<f64>,
    pub overage_permitted: Option<bool>,
    pub quota_reset_at: Option<u64>,
    pub timestamp_utc: Option<String>,
}

pub async fn read() -> Result<CopilotUsage, String> {
    let binary = resolve_gh_binary()?;
    let mut command = Command::new(binary);
    command
        .args(["api", USER_ENDPOINT])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(QUERY_TIMEOUT, command.output())
        .await
        .map_err(|_| "GitHub CLI timed out while loading Copilot usage".to_owned())?
        .map_err(|error| format!("Could not start GitHub CLI for Copilot usage: {error}"))?;
    if !output.status.success() {
        return Err(
            "GitHub CLI could not load Copilot usage. Check `gh auth status` and the account's Copilot access."
                .to_owned(),
        );
    }
    parse_usage(&output.stdout)
}

fn parse_usage(bytes: &[u8]) -> Result<CopilotUsage, String> {
    let usage: CopilotUsage = serde_json::from_slice(bytes).map_err(|error| {
        format!("GitHub Copilot returned an unsupported usage payload: {error}")
    })?;
    let quotas = &usage.quota_snapshots;
    if quotas.chat.is_none()
        && quotas.completions.is_none()
        && quotas.premium_interactions.is_none()
    {
        return Err("GitHub Copilot usage did not contain a supported quota".to_owned());
    }
    Ok(usage)
}

fn resolve_gh_binary() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("GITHUB_CLI_BINARY")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Ok(path);
    }
    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|directory| directory.join(if cfg!(windows) { "gh.exe" } else { "gh" }))
            .find(|candidate| candidate.is_file())
    }) {
        return Ok(path);
    }
    Err("GitHub CLI was not found. Install `gh` and run `gh auth login` first.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quota_snapshots() {
        let usage = parse_usage(
            br#"{
              "login": "octocat",
              "copilot_plan": "individual",
              "access_type_sku": "free_educational_quota",
              "quota_reset_date_utc": "2026-10-01T00:00:00.000Z",
              "quota_snapshots": {
                "chat": { "entitlement": 0, "remaining": 0, "percent_remaining": 100, "unlimited": true },
                "completions": { "entitlement": 0, "remaining": 0, "percent_remaining": 100, "unlimited": true },
                "premium_interactions": {
                  "entitlement": 200,
                  "remaining": 175.5,
                  "percent_remaining": 87.75,
                  "unlimited": false,
                  "overage_count": 0,
                  "overage_permitted": true,
                  "quota_reset_at": 0,
                  "timestamp_utc": "2026-09-01T09:24:18.929Z"
                }
              }
            }"#,
        )
        .expect("supported Copilot usage");

        assert_eq!(usage.login.as_deref(), Some("octocat"));
        let premium = usage
            .quota_snapshots
            .premium_interactions
            .expect("premium quota");
        assert_eq!(premium.entitlement, Some(200.0));
        assert_eq!(premium.remaining, Some(175.5));
        assert_eq!(premium.percent_remaining, Some(87.75));
        assert_eq!(premium.unlimited, Some(false));
    }

    #[test]
    fn rejects_payload_without_supported_quotas() {
        assert!(parse_usage(br#"{"quota_snapshots": {}}"#).is_err());
    }

    #[tokio::test]
    #[ignore = "requires a locally authenticated GitHub Copilot account"]
    async fn reads_live_usage() {
        let usage = read().await.expect("Copilot usage should be readable");
        assert!(usage.quota_snapshots.premium_interactions.is_some());
    }
}
