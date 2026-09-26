use std::time::{SystemTime, UNIX_EPOCH};

use crate::PublishError;

const AUTHORIZE_ENDPOINT: &str = "https://x.com/i/oauth2/authorize";
const TOKEN_ENDPOINT: &str = "https://api.x.com/2/oauth2/token";
const REDIRECT_URI: &str = "http://127.0.0.1:8792/callback";
const SCOPES: &str = "tweet.read tweet.write users.read offline.access";
const CLIENT_ID: Option<&str> = option_env!("VESPER_X_CLIENT_ID");

pub struct Authorization {
    pub url: String,
    client_id: String,
    grant: vesper_oauth::Authorization,
}

pub async fn authorization() -> Result<Authorization, PublishError> {
    let client_id = CLIENT_ID
        .filter(|client_id| !client_id.is_empty() && client_id.len() <= 256)
        .ok_or(PublishError::XAuthorization(
            "the X OAuth Client ID is not configured in this build",
        ))?;
    let grant = vesper_oauth::authorize(
        client_id,
        AUTHORIZE_ENDPOINT,
        TOKEN_ENDPOINT,
        REDIRECT_URI,
        SCOPES,
    )
    .await
    .map_err(oauth_error)?;
    Ok(Authorization {
        url: grant.url.clone(),
        client_id: client_id.to_owned(),
        grant,
    })
}

pub async fn authenticate(
    authorization: Authorization,
) -> Result<vesper_credentials::XCredentials, PublishError> {
    let token = vesper_oauth::authenticate(authorization.grant)
        .await
        .map_err(oauth_error)?;
    let refresh_token = token.refresh_token.ok_or(PublishError::XAuthorization(
        "X did not return a refresh token",
    ))?;
    Ok(vesper_credentials::XCredentials {
        client_id: authorization.client_id,
        access_token: token.access_token,
        refresh_token,
        expires_at: now().saturating_add(token.expires_in),
    })
}

pub async fn refresh(
    credentials: &vesper_credentials::XCredentials,
) -> Result<vesper_credentials::XCredentials, PublishError> {
    let token = vesper_oauth::refresh(
        &credentials.client_id,
        TOKEN_ENDPOINT,
        &credentials.refresh_token,
    )
    .await
    .map_err(oauth_error)?;
    Ok(vesper_credentials::XCredentials {
        client_id: credentials.client_id.clone(),
        access_token: token.access_token,
        refresh_token: token
            .refresh_token
            .unwrap_or_else(|| credentials.refresh_token.clone()),
        expires_at: now().saturating_add(token.expires_in),
    })
}

pub fn expires_soon(credentials: &vesper_credentials::XCredentials) -> bool {
    credentials.expires_at <= now().saturating_add(60)
}

fn oauth_error(error: vesper_oauth::Error) -> PublishError {
    match error {
        vesper_oauth::Error::Status(status) => PublishError::Status {
            provider: "X authorization",
            status,
        },
        vesper_oauth::Error::Callback(message) => PublishError::XAuthorization(message),
        vesper_oauth::Error::Request(_) => PublishError::Request("X authorization"),
        vesper_oauth::Error::Configuration | vesper_oauth::Error::Protocol => {
            PublishError::Protocol("X authorization")
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
