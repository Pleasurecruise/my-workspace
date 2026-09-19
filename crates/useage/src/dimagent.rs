use crate::cache::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
static CACHE: Cache<Result<DimAgentUsage, String>> = Cache::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct DimAgentUsage {
    #[serde(default)]
    pub plan_name: Option<String>,
    pub credits: DimAgentCredits,
    #[serde(default)]
    pub feature_meters: Vec<DimAgentFeatureMeter>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct DimAgentCredits {
    pub total_units: f64,
    pub used_units: f64,
    pub remaining_units: f64,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct DimAgentFeatureMeter {
    pub feature_key: String,
    pub total_remaining: f64,
    #[serde(default)]
    pub unit: Option<String>,
    pub unlimited: bool,
    pub total_allowance: f64,
    pub total_used: f64,
    #[serde(default)]
    pub period_end: Option<String>,
}

#[derive(Deserialize)]
struct UsageResponse {
    subscription: Option<Subscription>,
    credits: CreditsResponse,
    #[serde(default)]
    feature_meters: Vec<DimAgentFeatureMeter>,
}

#[derive(Deserialize)]
struct Subscription {
    product: Option<Product>,
    current_term: Option<CurrentTerm>,
}

#[derive(Deserialize)]
struct Product {
    name: Option<String>,
}

#[derive(Deserialize)]
struct CurrentTerm {
    end_at: Option<String>,
}

#[derive(Deserialize)]
struct CreditsResponse {
    #[serde(flatten)]
    credits: DimAgentCredits,
    subscription_bucket: Option<SubscriptionBucket>,
}

#[derive(Deserialize)]
struct SubscriptionBucket {
    expires_at: Option<String>,
}

pub async fn read() -> Result<DimAgentUsage, String> {
    CACHE.read(CACHE_TTL, read_fresh()).await
}

async fn read_fresh() -> Result<DimAgentUsage, String> {
    let binary = resolve_dim_binary()?;
    let result = tokio::time::timeout(RESPONSE_TIMEOUT, async {
        Command::new(&binary)
            .arg("usage")
            .arg("--json")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|error| {
                format!(
                    "Could not run DimAgent usage at {}: {error}",
                    binary.display()
                )
            })
    })
    .await
    .map_err(|_| {
        format!(
            "Timed out while reading DimAgent usage at {}",
            binary.display()
        )
    })?;
    let output = result?;
    if !output.status.success() {
        return Err(format!("DimAgent usage exited with {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|_| "DimAgent returned an unsupported usage payload".to_owned())?;
    parse_usage(&value)
}

fn parse_usage(value: &Value) -> Result<DimAgentUsage, String> {
    let response: UsageResponse = serde_json::from_value(value.clone())
        .map_err(|_| "DimAgent returned an unsupported usage payload".to_owned())?;
    let plan_name = response
        .subscription
        .as_ref()
        .and_then(|subscription| subscription.product.as_ref())
        .and_then(|product| product.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned);
    let mut credits = response.credits.credits;
    credits.expires_at = response
        .credits
        .subscription_bucket
        .and_then(|bucket| bucket.expires_at)
        .or_else(|| {
            response
                .subscription
                .and_then(|subscription| subscription.current_term)
                .and_then(|term| term.end_at)
        });
    Ok(DimAgentUsage {
        plan_name,
        credits,
        feature_meters: response.feature_meters,
    })
}

fn resolve_dim_binary() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DIM_BINARY").map(PathBuf::from)
        && path.is_file()
    {
        return Ok(path);
    }
    for candidate in bundled_candidates().into_iter().chain(path_candidates()) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("DimAgent CLI was not found. Install DimAgent Desktop or `npm install -g dimcode@latest`, then sign in with `dim auth login`.".to_owned())
}

fn bundled_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/DimAgent.app/Contents/Resources/runtime/cli/dim",
        ));
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(
                PathBuf::from(&home)
                    .join("Applications/DimAgent.app/Contents/Resources/runtime/cli/dim"),
            );
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(&local).join(r"Programs\DimAgent\resources\runtime\cli\dim.exe"),
            );
        }
        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            candidates.push(
                PathBuf::from(&program_files).join(r"DimAgent\resources\runtime\cli\dim.exe"),
            );
        }
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push(PathBuf::from("/opt/DimAgent/resources/runtime/cli/dim"));
        candidates.push(PathBuf::from("/usr/lib/DimAgent/resources/runtime/cli/dim"));
        if let Some(home) = std::env::var_os("HOME") {
            candidates
                .push(PathBuf::from(&home).join(".local/share/DimAgent/resources/runtime/cli/dim"));
        }
    }
    candidates
}

fn path_candidates() -> Vec<PathBuf> {
    let Some(paths) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    std::env::split_paths(&paths)
        .flat_map(|directory| {
            ["dim", "dimcode"].into_iter().map(move |name| {
                directory.join(if cfg!(windows) {
                    format!("{name}.exe")
                } else {
                    name.to_owned()
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_usage() {
        let usage = parse_usage(&serde_json::json!({
            "account_id": 1735,
            "subscription": {
                "product": { "name": "Lite套餐" },
                "current_term": { "end_at": "2026-10-18T16:44:55.794Z" }
            },
            "credits": {
                "subscription_bucket": { "expires_at": "2026-10-18T16:44:55.794Z" },
                "total_units": 11000,
                "used_units": 0,
                "remaining_units": 11000
            },
            "feature_meters": [
                {
                    "feature_key": "web_search",
                    "total_remaining": 500,
                    "unit": "call",
                    "unlimited": false,
                    "total_allowance": 500,
                    "total_used": 0,
                    "period_end": "2026-10-18T16:44:55.794Z"
                }
            ]
        }))
        .expect("valid DimAgent usage");

        assert_eq!(usage.plan_name.as_deref(), Some("Lite套餐"));
        assert_eq!(usage.credits.total_units, 11000.0);
        assert_eq!(usage.credits.remaining_units, 11000.0);
        assert_eq!(
            usage.credits.expires_at.as_deref(),
            Some("2026-10-18T16:44:55.794Z")
        );
        assert_eq!(usage.feature_meters.len(), 1);
        assert_eq!(usage.feature_meters[0].feature_key, "web_search");
        assert_eq!(usage.feature_meters[0].total_remaining, 500.0);
        let serialized = serde_json::to_value(&usage).expect("serialized usage");
        assert_eq!(serialized["credits"]["totalUnits"], 11000.0);
        assert_eq!(serialized["featureMeters"][0]["featureKey"], "web_search");
        assert!(serialized.get("account_id").is_none());
    }

    #[test]
    fn rejects_invalid_usage() {
        for value in [
            serde_json::json!({}),
            serde_json::json!(null),
            serde_json::json!({ "credits": {} }),
            serde_json::json!({
                "credits": { "total_units": 100, "used_units": 0, "remaining_units": 100 },
                "feature_meters": "invalid"
            }),
        ] {
            assert!(
                parse_usage(&value).is_err(),
                "accepted invalid usage: {value}"
            );
        }
    }

    #[test]
    fn preserves_zero_credits_and_nullable_metadata() {
        let usage = parse_usage(&serde_json::json!({
            "subscription": { "current_term": { "end_at": "2026-10-18T00:00:00Z" } },
            "credits": {
                "total_units": 0,
                "used_units": 0,
                "remaining_units": 0,
                "subscription_bucket": { "expires_at": null }
            }
        }))
        .expect("valid zero credits");
        let serialized = serde_json::to_value(usage).expect("serialized usage");
        assert_eq!(serialized["planName"], serde_json::Value::Null);
        assert_eq!(serialized["credits"]["totalUnits"], 0.0);
        assert_eq!(serialized["credits"]["expiresAt"], "2026-10-18T00:00:00Z");
        assert_eq!(serialized["featureMeters"], serde_json::json!([]));
    }

    #[tokio::test]
    #[ignore = "requires a locally authenticated DimAgent CLI"]
    async fn reads_live_usage() {
        let usage = read().await.expect("DimAgent usage should be readable");
        assert!(usage.credits.total_units >= 0.0);
    }
}
