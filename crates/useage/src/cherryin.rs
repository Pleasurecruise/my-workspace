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
const TOKEN_EXPIRY_BUFFER_MS: u64 = 60_000;
static SESSION_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Serialize)]
pub struct CherryInBalance {
    pub balance: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthConfig {
    #[serde(rename = "type")]
    kind: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
}

struct StoredOAuth {
    db_path: PathBuf,
    serialized: String,
    value: Value,
    config: OAuthConfig,
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
}

pub async fn read() -> Result<CherryInBalance, String> {
    let session_guard = SESSION_LOCK.lock().await;
    let read_task = tokio::task::spawn_blocking(read_oauth)
        .await
        .map_err(|error| format!("Could not join Cherry Studio credential read: {error}"))?;
    let mut stored = read_task?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Could not create CherryIN client: {error}"))?;
    let access_token = valid_access_token(&client, &mut stored, false).await?;
    let mut response = client
        .get(BALANCE_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|error| format!("Could not query CherryIN OAuth balance: {error}"))?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let access_token = valid_access_token(&client, &mut stored, true).await?;
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
    let balance = CherryInBalance {
        balance: response.data.quota / QUOTA_PER_UNIT,
    };
    drop(session_guard);
    Ok(balance)
}

fn read_oauth() -> Result<StoredOAuth, String> {
    let db_path = oauth_db_path()?;
    let connection = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| {
        format!(
            "Could not open Cherry Studio database {}: {error}",
            db_path.display()
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
    let config: OAuthConfig = serde_json::from_value(value.clone())
        .map_err(|error| format!("Could not decode Cherry Studio OAuth configuration: {error}"))?;
    if config.kind != "oauth" {
        return Err("Cherry Studio is not signed in to CherryIN with OAuth".to_owned());
    }
    Ok(StoredOAuth {
        db_path,
        serialized: auth_config,
        value,
        config,
    })
}

async fn valid_access_token(
    client: &reqwest::Client,
    stored: &mut StoredOAuth,
    force_refresh: bool,
) -> Result<String, String> {
    let access_token = stored
        .config
        .access_token
        .as_deref()
        .filter(|token| !token.trim().is_empty());
    let refresh_needed = token_needs_refresh(&stored.config)?;
    if let (false, Some(access_token)) = (force_refresh || refresh_needed, access_token) {
        return Ok(access_token.to_owned());
    }

    let refresh_token = stored
        .config
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
    save_tokens(stored, tokens).await
}

fn token_needs_refresh(config: &OAuthConfig) -> Result<bool, String> {
    let now = unix_time_millis()?;
    Ok(config
        .expires_at
        .is_some_and(|expires_at| expires_at <= now.saturating_add(TOKEN_EXPIRY_BUFFER_MS)))
}

async fn save_tokens(stored: &mut StoredOAuth, tokens: TokenResponse) -> Result<String, String> {
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
    let db_path = stored.db_path.clone();
    let previous = stored.serialized.clone();
    let next = serialized.clone();
    let update_task = tokio::task::spawn_blocking(move || {
        let connection = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| {
            format!(
                "Could not open Cherry Studio database {} for OAuth refresh: {error}",
                db_path.display()
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
    .map_err(|error| format!("Could not join Cherry Studio credential update: {error}"))?;
    let updated = update_task?;
    if updated != 1 {
        return Err(
            "Cherry Studio's CherryIN OAuth session changed while it was refreshing; retry Dashboard refresh"
                .to_owned(),
        );
    }
    stored.serialized = serialized;
    stored.config = serde_json::from_value(stored.value.clone()).map_err(|error| {
        format!("Could not decode refreshed Cherry Studio OAuth session: {error}")
    })?;
    Ok(access_token.to_owned())
}

fn oauth_db_path() -> Result<PathBuf, String> {
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
    fn decodes_oauth_config() {
        let config: OAuthConfig = serde_json::from_value(serde_json::json!({
            "type": "oauth",
            "accessToken": "oauth-access-token",
            "refreshToken": "oauth-refresh-token",
            "expiresAt": 4_102_444_800_000_u64
        }))
        .expect("valid OAuth configuration");

        assert_eq!(config.kind, "oauth");
        assert_eq!(config.access_token.as_deref(), Some("oauth-access-token"));
        assert_eq!(config.refresh_token.as_deref(), Some("oauth-refresh-token"));
    }

    #[test]
    fn refreshes_before_expiry() {
        let config = OAuthConfig {
            kind: "oauth".to_owned(),
            access_token: Some("access-token".to_owned()),
            refresh_token: Some("refresh-token".to_owned()),
            expires_at: Some(
                unix_time_millis().expect("current Unix time") + TOKEN_EXPIRY_BUFFER_MS - 1,
            ),
        };

        assert!(token_needs_refresh(&config).expect("current Unix time"));
    }

    #[test]
    fn converts_quota_units() {
        let response: BalanceResponse = serde_json::from_value(serde_json::json!({
            "success": true,
            "data": { "quota": 37_500_000, "used_quota": 12_500_000 }
        }))
        .expect("valid balance response");

        assert_eq!(response.data.quota / QUOTA_PER_UNIT, 75.0);
    }

    #[tokio::test]
    #[ignore = "requires Cherry Studio OAuth and network access"]
    async fn reads_live_balance() {
        let balance = read().await.expect("CherryIN balance should be readable");
        assert!(balance.balance >= 0.0);
    }
}
