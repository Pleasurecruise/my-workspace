use serde::{Deserialize, Serialize};
use std::time::Duration;

const USAGE_URL: &str = "https://opencode.ai/zen/go/v1/usage";

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenCodeUsage {
    pub usage: UsageWindows,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UsageWindows {
    pub rolling: UsageWindow,
    pub weekly: UsageWindow,
    pub monthly: UsageWindow,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub status: String,
    pub percent: f64,
    pub resets_at: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

pub async fn read() -> Result<OpenCodeUsage, String> {
    let api_key = crate::auth::api_key("opencode-go").await?;
    let response = reqwest::Client::new()
        .get(USAGE_URL)
        .bearer_auth(api_key)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|error| format!("Could not query OpenCode Go usage: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read OpenCode Go response: {error}"))?;
    if !status.is_success() {
        let message = match serde_json::from_str::<ErrorResponse>(&body) {
            Ok(response) => match response.error {
                Some(error) => match error.message {
                    Some(message) => message,
                    None => format!("HTTP {status}"),
                },
                None => format!("HTTP {status}"),
            },
            Err(_) => format!("HTTP {status}"),
        };
        return Err(format!("OpenCode Go usage request failed: {message}"));
    }
    serde_json::from_str(&body)
        .map_err(|error| format!("OpenCode Go returned an unsupported usage payload: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usage_windows() {
        let usage: OpenCodeUsage = serde_json::from_value(serde_json::json!({
            "usage": {
                "rolling": { "status": "ok", "percent": 12.5, "resetsAt": "2026-08-23T12:00:00.000Z" },
                "weekly": { "status": "ok", "percent": 25, "resetsAt": "2026-08-30T12:00:00.000Z" },
                "monthly": { "status": "rate-limited", "percent": 100, "resetsAt": "2026-09-01T00:00:00.000Z" }
            }
        }))
        .expect("valid usage response");

        assert_eq!(usage.usage.rolling.percent, 12.5);
        assert_eq!(usage.usage.monthly.status, "rate-limited");
    }

    #[tokio::test]
    #[ignore = "requires a locally authenticated OpenCode Go subscription"]
    async fn reads_usage_from_local_opencode() {
        let usage = read().await.expect("OpenCode Go usage should be readable");
        assert!(!usage.usage.rolling.resets_at.is_empty());
    }
}
