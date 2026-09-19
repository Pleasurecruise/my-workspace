use serde::{Deserialize, Serialize};
use std::time::Duration;

const USAGE_URL: &str = "https://tokenflux.dev/v1/usage";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TokenFluxUsage {
    pub remaining: f64,
    pub unit: String,
    #[serde(rename = "planName")]
    pub plan_name: String,
    #[serde(rename = "isValid")]
    pub is_valid: bool,
    pub mode: String,
    pub billing: TokenFluxBilling,
    pub subscription: TokenFluxSubscription,
    pub usage: TokenFluxUsageDetail,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TokenFluxBilling {
    pub available: bool,
    pub mode: String,
    pub plan_id: i64,
    pub plan_name: String,
    pub preferred_subscription_id: Option<i64>,
    pub remaining: f64,
    pub source: Option<String>,
    pub subscription_id: i64,
    pub unit: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TokenFluxSubscription {
    pub daily_limit_usd: f64,
    pub daily_usage_usd: f64,
    pub expires_at: Option<String>,
    pub id: i64,
    pub monthly_limit_usd: f64,
    pub monthly_usage_usd: f64,
    pub plan_id: i64,
    pub status: String,
    pub weekly_limit_usd: Option<f64>,
    pub weekly_usage_usd: f64,
    pub weekly_window_start: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TokenFluxUsageDetail {
    #[serde(default)]
    pub average_duration_ms: f64,
    #[serde(default)]
    pub rpm: f64,
    #[serde(default)]
    pub tpm: f64,
    #[serde(default)]
    pub today: TokenFluxWindow,
    #[serde(default)]
    pub total: TokenFluxWindow,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TokenFluxWindow {
    #[serde(default)]
    pub actual_cost: f64,
    #[serde(default)]
    pub cache_creation_tokens: i64,
    #[serde(default)]
    pub cache_read_tokens: i64,
    #[serde(default)]
    pub cost: f64,
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub requests: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

pub async fn read() -> Result<TokenFluxUsage, String> {
    let api_key = crate::auth::api_key("tokenflux").await?;
    let response = reqwest::Client::new()
        .get(USAGE_URL)
        .bearer_auth(api_key)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|error| format!("Could not query TokenFlux usage: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read TokenFlux response: {error}"))?;
    if !status.is_success() {
        return Err(format!("TokenFlux usage request failed: HTTP {status}"));
    }
    serde_json::from_str(&body)
        .map_err(|_| "TokenFlux returned an unsupported usage payload".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_usage() {
        let usage: TokenFluxUsage = serde_json::from_value(serde_json::json!({
            "remaining": 100,
            "unit": "推理积分",
            "planName": "Lite",
            "isValid": true,
            "mode": "unrestricted",
            "billing": {
                "available": true,
                "mode": "auto",
                "plan_id": 2,
                "plan_name": "Lite",
                "preferred_subscription_id": null,
                "remaining": 100,
                "source": "subscription",
                "subscription_id": 13812,
                "unit": "推理积分"
            },
            "subscription": {
                "daily_limit_usd": 100,
                "daily_usage_usd": 0,
                "expires_at": "2026-09-20T00:00:48.288048+08:00",
                "id": 13812,
                "monthly_limit_usd": 500,
                "monthly_usage_usd": 220.3323027,
                "plan_id": 2,
                "status": "active",
                "weekly_limit_usd": null,
                "weekly_usage_usd": 0,
                "weekly_window_start": null
            },
            "usage": {
                "average_duration_ms": 15962.769370460048,
                "rpm": 0,
                "tpm": 0,
                "today": {
                    "actual_cost": 0,
                    "cache_creation_tokens": 0,
                    "cache_read_tokens": 0,
                    "cost": 0,
                    "input_tokens": 0,
                    "output_tokens": 0,
                    "requests": 0,
                    "total_tokens": 0
                },
                "total": {
                    "actual_cost": 220.3323027,
                    "cache_creation_tokens": 0,
                    "cache_read_tokens": 79477120,
                    "cost": 4.406646054,
                    "input_tokens": 10489368,
                    "output_tokens": 3490349,
                    "requests": 1652,
                    "total_tokens": 93456837
                }
            }
        }))
        .expect("valid usage response");

        assert_eq!(usage.billing.plan_name, "Lite");
        assert_eq!(usage.billing.remaining, 100.0);
        assert_eq!(usage.subscription.monthly_limit_usd, 500.0);
        assert_eq!(usage.subscription.monthly_usage_usd, 220.3323027);
        assert_eq!(usage.subscription.weekly_limit_usd, None);
        assert_eq!(usage.usage.today.requests, 0);
        assert_eq!(usage.usage.total.total_tokens, 93456837);
        assert_eq!(usage.usage.total.cache_read_tokens, 79477120);
        let serialized = serde_json::to_value(usage).expect("serialized usage");
        assert_eq!(serialized["planName"], "Lite");
        assert_eq!(serialized["subscription"]["monthlyLimitUsd"], 500.0);
        assert_eq!(
            serialized["subscription"]["weeklyLimitUsd"],
            serde_json::Value::Null
        );
        assert!(
            serialized["subscription"]
                .get("monthly_limit_usd")
                .is_none()
        );
    }

    #[test]
    fn tolerates_null_subscription_fields() {
        let usage: TokenFluxUsage = serde_json::from_value(serde_json::json!({
            "remaining": 50,
            "unit": "推理积分",
            "planName": "Lite",
            "isValid": true,
            "mode": "unrestricted",
            "billing": {
                "available": true,
                "mode": "auto",
                "plan_id": 2,
                "plan_name": "Lite",
                "preferred_subscription_id": null,
                "remaining": 50,
                "source": null,
                "subscription_id": 13812,
                "unit": "推理积分"
            },
            "subscription": {
                "daily_limit_usd": 100,
                "daily_usage_usd": 10,
                "expires_at": null,
                "id": 13812,
                "monthly_limit_usd": 500,
                "monthly_usage_usd": 50,
                "plan_id": 2,
                "status": "active",
                "weekly_limit_usd": null,
                "weekly_usage_usd": 0,
                "weekly_window_start": null
            },
            "usage": { "today": {}, "total": {} }
        }))
        .expect("nullable subscription fields");

        assert_eq!(usage.billing.source, None);
        assert_eq!(usage.subscription.expires_at, None);
        assert_eq!(usage.usage.today.requests, 0);
    }

    #[tokio::test]
    #[ignore = "requires a locally configured TokenFlux API key"]
    async fn reads_live_usage() {
        let usage = read().await.expect("TokenFlux usage should be readable");
        assert!(usage.billing.remaining >= 0.0);
    }
}
