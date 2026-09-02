use crate::{CredentialError, SERVICE, Stored};

#[cfg(not(debug_assertions))]
const ACCOUNT: &str = "qq-music";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct QqMusicCredentials {
    pub cookie: String,
}

pub fn qq_music() -> Result<Stored<QqMusicCredentials>, CredentialError> {
    #[cfg(debug_assertions)]
    return read_development();
    #[cfg(not(debug_assertions))]
    return read_store();
}

pub fn save_qq_music(credentials: QqMusicCredentials) -> Result<(), CredentialError> {
    validate(&credentials.cookie)?;
    #[cfg(debug_assertions)]
    return save_development(&credentials);
    #[cfg(not(debug_assertions))]
    {
        let encoded = serde_json::to_string(&credentials)?;
        keyring::Entry::new(SERVICE, ACCOUNT)?.set_password(&encoded)?;
        Ok(())
    }
}

#[cfg(not(debug_assertions))]
fn read_store() -> Result<Stored<QqMusicCredentials>, CredentialError> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    match entry.get_password() {
        Ok(encoded) => {
            let credentials: QqMusicCredentials = serde_json::from_str(&encoded)?;
            validate(&credentials.cookie)?;
            Ok(Stored::Ready(credentials))
        }
        Err(keyring::Error::NoEntry) => Ok(Stored::Missing),
        Err(error) => Err(CredentialError::Store(error)),
    }
}

#[cfg(debug_assertions)]
fn development_path() -> Result<std::path::PathBuf, CredentialError> {
    dirs::data_local_dir()
        .map(|directory| directory.join(SERVICE).join("development-qq-music.json"))
        .ok_or(CredentialError::DevelopmentDataDirectory)
}

#[cfg(debug_assertions)]
fn read_development() -> Result<Stored<QqMusicCredentials>, CredentialError> {
    let path = development_path()?;
    match crate::development_storage::read::<QqMusicCredentials>(&path)? {
        Stored::Ready(credentials) => {
            validate(&credentials.cookie)?;
            Ok(Stored::Ready(credentials))
        }
        Stored::Missing => Ok(Stored::Missing),
    }
}

#[cfg(debug_assertions)]
fn save_development(credentials: &QqMusicCredentials) -> Result<(), CredentialError> {
    let path = development_path()?;
    crate::development_storage::save(&path, credentials)
}

fn validate(cookie: &str) -> Result<(), CredentialError> {
    if cookie.trim().is_empty() {
        return Err(CredentialError::Empty("QQ Music cookie"));
    }
    if !cookie.split(';').any(|part| {
        let key = part.split_once('=').map(|(key, _)| key.trim());
        matches!(key, Some("uin" | "qqmusic_uin" | "wxuin" | "p_uin"))
    }) {
        return Err(CredentialError::Empty("QQ Music uin cookie"));
    }
    if !cookie.split(';').any(|part| {
        let key = part.split_once('=').map(|(key, _)| key.trim());
        matches!(
            key,
            Some("qm_keyst" | "qqmusic_key" | "music_key" | "wxskey")
        )
    }) {
        return Err(CredentialError::Empty("QQ Music playback cookie"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn requires_identity_and_playback_cookie_fields() {
        assert!(validate("").is_err());
        assert!(validate("uin=123").is_err());
        assert!(validate("qm_keyst=secret").is_err());
        assert!(validate("uin=123; qm_keyst=secret").is_ok());
    }
}
