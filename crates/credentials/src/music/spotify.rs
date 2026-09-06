use crate::store;
use crate::{CredentialError, Stored};

const ACCOUNT: &str = "spotify-music";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct SpotifyCredentials {
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
    fn rejects_incomplete_spotify_credentials() {
        assert!(validate("", "playback").is_err());
        assert!(validate("web", "").is_err());
        assert!(validate("web", "playback").is_ok());
    }
}
