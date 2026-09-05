#[cfg(debug_assertions)]
use crate::SERVICE;
#[cfg(not(debug_assertions))]
use crate::store;
use crate::{CredentialError, Stored};

#[cfg(not(debug_assertions))]
const ACCOUNT: &str = "spotify-music";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct SpotifyCredentials {
    pub web_refresh_token: String,
    pub playback_refresh_token: String,
}

pub fn spotify() -> Result<Stored<SpotifyCredentials>, CredentialError> {
    #[cfg(debug_assertions)]
    return read_development();
    #[cfg(not(debug_assertions))]
    return read_store();
}

pub fn save_spotify(credentials: SpotifyCredentials) -> Result<(), CredentialError> {
    validate(
        &credentials.web_refresh_token,
        &credentials.playback_refresh_token,
    )?;
    #[cfg(debug_assertions)]
    return save_development(&credentials);
    #[cfg(not(debug_assertions))]
    {
        let encoded = serde_json::to_string(&credentials)?;
        store::save(ACCOUNT, &encoded)?;
        Ok(())
    }
}

#[cfg(not(debug_assertions))]
fn read_store() -> Result<Stored<SpotifyCredentials>, CredentialError> {
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

#[cfg(debug_assertions)]
fn development_path() -> Result<std::path::PathBuf, CredentialError> {
    dirs::data_local_dir()
        .map(|directory| directory.join(SERVICE).join("development-spotify.json"))
        .ok_or(CredentialError::DevelopmentDataDirectory)
}

#[cfg(debug_assertions)]
fn read_development() -> Result<Stored<SpotifyCredentials>, CredentialError> {
    let path = development_path()?;
    match crate::development_storage::read::<SpotifyCredentials>(&path)? {
        Stored::Ready(credentials) => {
            validate(
                &credentials.web_refresh_token,
                &credentials.playback_refresh_token,
            )?;
            Ok(Stored::Ready(credentials))
        }
        Stored::Missing => Ok(Stored::Missing),
    }
}

#[cfg(debug_assertions)]
fn save_development(credentials: &SpotifyCredentials) -> Result<(), CredentialError> {
    let path = development_path()?;
    crate::development_storage::save(&path, credentials)
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
