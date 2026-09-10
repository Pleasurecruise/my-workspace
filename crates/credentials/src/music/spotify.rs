use crate::store;
use crate::{CredentialError, Stored};

const ACCOUNT: &str = "spotify-music";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct SpotifyCredentials {
    #[serde(default)]
    pub web_client_id: Option<String>,
    pub web_refresh_token: String,
    pub playback_refresh_token: String,
}

pub fn save_spotify(credentials: SpotifyCredentials) -> Result<(), CredentialError> {
    validate(
        &credentials.web_refresh_token,
        &credentials.playback_refresh_token,
    )?;
    let encoded = serde_json::to_string(&credentials)?;
    store::save(ACCOUNT, &encoded)?;
    Ok(())
}

pub fn spotify() -> Result<Stored<SpotifyCredentials>, CredentialError> {
    match store::read(ACCOUNT)? {
        Stored::Ready(encoded) => {
            let credentials: SpotifyCredentials = serde_json::from_str(&encoded)?;
            validate(
                &credentials.web_refresh_token,
                &credentials.playback_refresh_token,
            )?;
            Ok(Stored::Ready(credentials))
        }
        Stored::Missing => Ok(Stored::Missing),
    }
}

fn validate(web_refresh_token: &str, playback_refresh_token: &str) -> Result<(), CredentialError> {
    if web_refresh_token.trim().is_empty() {
        return Err(CredentialError::Empty("Spotify Web refresh token"));
    }
    if playback_refresh_token.trim().is_empty() {
        return Err(CredentialError::Empty("Spotify playback refresh token"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn legacy_spotify_grants_keep_shared_access_and_personal_grants_retain_their_client() {
        let old: super::SpotifyCredentials = serde_json::from_str(
            r#"{"web_refresh_token":"web","playback_refresh_token":"playback"}"#,
        )
        .unwrap();
        assert!(old.web_client_id.is_none());
        let personal = super::SpotifyCredentials {
            web_client_id: Some("0123456789abcdef0123456789abcdef".to_owned()),
            ..old
        };
        let decoded: super::SpotifyCredentials =
            serde_json::from_str(&serde_json::to_string(&personal).unwrap()).unwrap();
        assert_eq!(decoded.web_client_id, personal.web_client_id);
    }

    #[test]
    fn rejects_incomplete_spotify_credentials() {
        assert!(validate("", "playback").is_err());
        assert!(validate("web", "").is_err());
        assert!(validate("web", "playback").is_ok());
    }
}
