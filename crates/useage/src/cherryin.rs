use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BALANCE_URL: &str = "https://open.cherryin.ai/api/v1/oauth/balance";
const QUOTA_PER_UNIT: f64 = 500_000.0;

#[derive(Debug, Serialize)]
pub struct CherryInBalance {
    pub balance: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthConfiguration {
    #[serde(rename = "type")]
    kind: String,
    access_token: Option<String>,
    expires_at: Option<u64>,
}

#[derive(Deserialize)]
struct BalanceResponse {
    success: bool,
    data: BalanceData,
}

#[derive(Deserialize)]
struct BalanceData {
    quota: f64,
    #[serde(rename = "used_quota")]
    _used_quota: f64,
}

pub async fn read() -> Result<CherryInBalance, String> {
    let configuration = tokio::task::spawn_blocking(read_cherry_studio_oauth_configuration)
        .await
        .map_err(|error| format!("Could not join Cherry Studio credential read: {error}"))??;
    let access_token = valid_access_token(&configuration)?;
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Could not create CherryIN client: {error}"))?
        .get(BALANCE_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| format!("Could not query CherryIN OAuth balance: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "CherryIN OAuth balance request failed: HTTP {status}"
        ));
    }
    let response: BalanceResponse = response
        .json()
        .await
        .map_err(|error| format!("CherryIN returned an unsupported balance response: {error}"))?;
    if !response.success {
        return Err("CherryIN OAuth balance request was not successful".to_owned());
    }
    Ok(CherryInBalance {
        balance: response.data.quota / QUOTA_PER_UNIT,
    })
}

fn read_cherry_studio_oauth_configuration() -> Result<OAuthConfiguration, String> {
    let database_path = cherry_studio_database_path()?;
    let connection = Connection::open_with_flags(
        &database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| {
        format!(
            "Could not open Cherry Studio database {}: {error}",
            database_path.display()
        )
    })?;
    let auth_config: String = connection
        .query_row(
            "SELECT auth_config FROM user_provider WHERE lower(provider_id) = 'cherryin' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not read Cherry Studio OAuth configuration: {error}"))?;
    let configuration: OAuthConfiguration = serde_json::from_str(&auth_config)
        .map_err(|error| format!("Could not decode Cherry Studio OAuth configuration: {error}"))?;
    if configuration.kind != "oauth" {
        return Err("Cherry Studio is not signed in to CherryIN with OAuth".to_owned());
    }
    Ok(configuration)
}

fn valid_access_token(configuration: &OAuthConfiguration) -> Result<String, String> {
    let access_token = configuration
        .access_token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| "Cherry Studio does not contain a CherryIN OAuth access token".to_owned())?;
    if configuration
        .expires_at
        .is_some_and(|expires_at| expires_at <= unix_time_millis())
    {
        return Err(
            "Cherry Studio's CherryIN OAuth session has expired; refresh it in Cherry Studio"
                .to_owned(),
        );
    }
    Ok(access_token.to_owned())
}

fn cherry_studio_database_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "linux")]
    let application_data = dirs::config_dir();
    #[cfg(not(target_os = "linux"))]
    let application_data = dirs::data_dir();
    application_data
        .map(|directory| directory.join("CherryStudio/Data/cherrystudio.sqlite"))
        .ok_or_else(|| {
            "Could not locate the operating-system application data directory".to_owned()
        })
}

fn unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_cherry_studio_oauth_configuration() {
        let configuration: OAuthConfiguration = serde_json::from_value(serde_json::json!({
            "type": "oauth",
            "accessToken": "oauth-access-token",
            "refreshToken": "oauth-refresh-token",
            "expiresAt": 4_102_444_800_000_u64
        }))
        .expect("valid OAuth configuration");

        assert_eq!(configuration.kind, "oauth");
        assert_eq!(
            configuration.access_token.as_deref(),
            Some("oauth-access-token")
        );
    }

    #[test]
    fn converts_account_quota_with_cherryin_unit() {
        let response: BalanceResponse = serde_json::from_value(serde_json::json!({
            "success": true,
            "data": { "quota": 37_500_000, "used_quota": 12_500_000 }
        }))
        .expect("valid balance response");

        assert_eq!(response.data.quota / QUOTA_PER_UNIT, 75.0);
    }

    #[tokio::test]
    #[ignore = "requires Cherry Studio OAuth and network access"]
    async fn reads_balance_from_cherry_studio_oauth() {
        let balance = read().await.expect("CherryIN balance should be readable");
        assert!(balance.balance >= 0.0);
    }
}
