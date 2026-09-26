use crate::{Error, Result};
pub use vesper_oauth::Authorization;

const AUTHORIZE_ENDPOINT: &str = "https://accounts.spotify.com/authorize";
const TOKEN_ENDPOINT: &str = "https://accounts.spotify.com/api/token";
pub(crate) const WEB_CLIENT_ID: &str = "d420a117a32841c2b3474932e49fb54b";
pub(crate) const PLAYBACK_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const WEB_SCOPES: &str = "user-library-read user-read-private";
const PLAYBACK_SCOPES: &str = "app-remote-control streaming user-modify-playback-state user-read-currently-playing user-read-playback-state user-read-private";

pub(crate) struct AccessToken {
    pub value: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

pub struct GrantToken {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn web_authorization(client_id: Option<&str>) -> Result<Authorization> {
    let client_id = client_id.unwrap_or(WEB_CLIENT_ID);
    if client_id.len() != 32 || !client_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::Authentication(
            "Spotify Client ID must contain 32 hexadecimal characters".to_owned(),
        ));
    }
    vesper_oauth::authorize(
        client_id,
        AUTHORIZE_ENDPOINT,
        TOKEN_ENDPOINT,
        "http://127.0.0.1:8989/login",
        WEB_SCOPES,
    )
    .await
    .map_err(|error| Error::Authentication(format!("Spotify: {error}")))
}

pub async fn playback_authorization() -> Result<Authorization> {
    vesper_oauth::authorize(
        PLAYBACK_CLIENT_ID,
        AUTHORIZE_ENDPOINT,
        TOKEN_ENDPOINT,
        "http://127.0.0.1:8898/login",
        PLAYBACK_SCOPES,
    )
    .await
    .map_err(|error| Error::Authentication(format!("Spotify: {error}")))
}

pub async fn authenticate(authorization: Authorization) -> Result<GrantToken> {
    let token = vesper_oauth::authenticate(authorization)
        .await
        .map_err(|error| Error::Authentication(format!("Spotify: {error}")))?;
    let refresh_token = token.refresh_token.ok_or_else(|| {
        Error::Authentication("Spotify did not return a refresh token".to_owned())
    })?;
    Ok(GrantToken {
        access_token: token.access_token,
        refresh_token,
    })
}

pub(crate) async fn refresh(client_id: &str, refresh_token: &str) -> Result<AccessToken> {
    let token = vesper_oauth::refresh(client_id, TOKEN_ENDPOINT, refresh_token)
        .await
        .map_err(|error| Error::Authentication(format!("Spotify: {error}")))?;
    Ok(AccessToken {
        value: token.access_token,
        expires_in: token.expires_in,
        refresh_token: token.refresh_token,
    })
}

#[cfg(test)]
mod tests {
    use super::WEB_CLIENT_ID;

    #[tokio::test]
    async fn validates_personal_client() {
        assert!(super::web_authorization(Some("invalid")).await.is_err());
        let client_id = "0123456789abcdef0123456789abcdef";
        let authorization = super::web_authorization(Some(client_id)).await.unwrap();
        let url = reqwest::Url::parse(&authorization.url).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(query.get("client_id").unwrap(), client_id);
        assert_eq!(
            query.get("redirect_uri").unwrap(),
            "http://127.0.0.1:8989/login"
        );
        assert_eq!(query.get("scope").unwrap(), super::WEB_SCOPES);
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
    }

    #[test]
    fn shared_web_client_is_stable() {
        assert_eq!(WEB_CLIENT_ID.len(), 32);
    }
}
