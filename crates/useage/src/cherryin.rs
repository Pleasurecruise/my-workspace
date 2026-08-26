use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const BALANCE_URL: &str = "https://open.cherryin.ai/api/v1/oauth/balance";
const TOKEN_URL: &str = "https://open.cherryin.ai/oauth2/token";
const CLIENT_ID: &str = "2a348c87-bae1-4756-a62f-b2e97200fd6d";
const QUOTA_PER_UNIT: f64 = 500_000.0;
const TOKEN_EXPIRY_BUFFER_MILLIS: u64 = 60_000;
static SESSION_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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
    refresh_token: Option<String>,
    expires_at: Option<u64>,
}

struct StoredOAuthConfiguration {
    database_path: PathBuf,
    serialized: String,
    value: Value,
    configuration: OAuthConfiguration,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
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
    let _session = SESSION_LOCK.lock().await;
    let mut configuration = tokio::task::spawn_blocking(read_cherry_studio_oauth_configuration)
        .await
        .map_err(|error| format!("Could not join Cherry Studio credential read: {error}"))??;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Could not create CherryIN client: {error}"))?;
    let access_token = valid_access_token(&client, &mut configuration, false).await?;
    let mut response = client
        .get(BALANCE_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|error| format!("Could not query CherryIN OAuth balance: {error}"))?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let access_token = valid_access_token(&client, &mut configuration, true).await?;
        response = client
            .get(BALANCE_URL)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| format!("Could not retry CherryIN OAuth balance: {error}"))?;
    }
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

fn read_cherry_studio_oauth_configuration() -> Result<StoredOAuthConfiguration, String> {
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
    let value: Value = serde_json::from_str(&auth_config)
        .map_err(|error| format!("Could not decode Cherry Studio OAuth configuration: {error}"))?;
    let configuration: OAuthConfiguration = serde_json::from_value(value.clone())
        .map_err(|error| format!("Could not decode Cherry Studio OAuth configuration: {error}"))?;
    if configuration.kind != "oauth" {
        return Err("Cherry Studio is not signed in to CherryIN with OAuth".to_owned());
    }
    Ok(StoredOAuthConfiguration {
        database_path,
        serialized: auth_config,
        value,
        configuration,
    })
}

async fn valid_access_token(
    client: &reqwest::Client,
    stored: &mut StoredOAuthConfiguration,
    force_refresh: bool,
) -> Result<String, String> {
    let access_token = stored
        .configuration
        .access_token
        .as_deref()
        .filter(|token| !token.trim().is_empty());
    if !force_refresh
        && !access_token_needs_refresh(&stored.configuration)?
        && let Some(access_token) = access_token
    {
        return Ok(access_token.to_owned());
    }

    let refresh_token = stored
        .configuration
        .refresh_token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| {
            "Cherry Studio's CherryIN OAuth session cannot be refreshed; sign in again in Cherry Studio"
                .to_owned()
        })?;
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|error| format!("Could not refresh CherryIN OAuth session: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "CherryIN OAuth session refresh failed: HTTP {status}; sign in again in Cherry Studio if retrying does not help"
        ));
    }
    let tokens: TokenResponse = response
        .json()
        .await
        .map_err(|error| format!("CherryIN returned an unsupported token response: {error}"))?;
    persist_refreshed_tokens(stored, tokens).await
}

fn access_token_needs_refresh(configuration: &OAuthConfiguration) -> Result<bool, String> {
    let now = unix_time_millis()?;
    Ok(configuration
        .expires_at
        .is_some_and(|expires_at| expires_at <= now.saturating_add(TOKEN_EXPIRY_BUFFER_MILLIS)))
}

async fn persist_refreshed_tokens(
    stored: &mut StoredOAuthConfiguration,
    tokens: TokenResponse,
) -> Result<String, String> {
    let access_token = tokens.access_token.trim();
    if access_token.is_empty() {
        return Err("CherryIN token refresh returned an empty access token".to_owned());
    }
    let object = stored
        .value
        .as_object_mut()
        .ok_or_else(|| "Cherry Studio OAuth configuration is not an object".to_owned())?;
    object.insert(
        "accessToken".to_owned(),
        Value::String(access_token.to_owned()),
    );
    if let Some(refresh_token) = tokens
        .refresh_token
        .filter(|token| !token.trim().is_empty())
    {
        object.insert("refreshToken".to_owned(), Value::String(refresh_token));
    }
    if let Some(expires_in) = tokens.expires_in {
        object.insert(
            "expiresAt".to_owned(),
            Value::from(unix_time_millis()?.saturating_add(expires_in.saturating_mul(1_000))),
        );
    } else {
        object.remove("expiresAt");
    }
    let serialized = serde_json::to_string(&stored.value).map_err(|error| {
        format!("Could not encode refreshed Cherry Studio OAuth session: {error}")
    })?;
    let database_path = stored.database_path.clone();
    let previous = stored.serialized.clone();
    let next = serialized.clone();
    let updated = tokio::task::spawn_blocking(move || {
        let connection = Connection::open_with_flags(
            &database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| {
            format!(
                "Could not open Cherry Studio database {} for OAuth refresh: {error}",
                database_path.display()
            )
        })?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| format!("Could not configure Cherry Studio database timeout: {error}"))?;
        connection
            .execute(
                "UPDATE user_provider SET auth_config = ?1 WHERE lower(provider_id) = 'cherryin' AND auth_config = ?2",
                (&next, &previous),
            )
            .map_err(|error| format!("Could not persist refreshed CherryIN OAuth session: {error}"))
    })
    .await
    .map_err(|error| format!("Could not join Cherry Studio credential update: {error}"))??;
    if updated != 1 {
        return Err(
            "Cherry Studio's CherryIN OAuth session changed while it was refreshing; retry Dashboard refresh"
                .to_owned(),
        );
    }
    stored.serialized = serialized;
    stored.configuration = serde_json::from_value(stored.value.clone()).map_err(|error| {
        format!("Could not decode refreshed Cherry Studio OAuth session: {error}")
    })?;
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

fn unix_time_millis() -> Result<u64, String> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System time is before the Unix epoch: {error}"))?
        .as_millis();
    u64::try_from(milliseconds)
        .map_err(|_| "System time does not fit the CherryIN OAuth timestamp format".to_owned())
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
        assert_eq!(
            configuration.refresh_token.as_deref(),
            Some("oauth-refresh-token")
        );
    }

    #[test]
    fn refreshes_tokens_before_the_expiry_buffer() {
        let configuration = OAuthConfiguration {
            kind: "oauth".to_owned(),
            access_token: Some("access-token".to_owned()),
            refresh_token: Some("refresh-token".to_owned()),
            expires_at: Some(
                unix_time_millis().expect("current Unix time") + TOKEN_EXPIRY_BUFFER_MILLIS - 1,
            ),
        };

        assert!(access_token_needs_refresh(&configuration).expect("current Unix time"));
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
