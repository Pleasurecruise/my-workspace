use crate::store;
use crate::{CredentialError, Stored};

const ACCOUNT: &str = "qq-music";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct QqMusicCredentials {
    pub cookie: String,
}

pub fn save_qq_music(credentials: QqMusicCredentials) -> Result<(), CredentialError> {
    validate(&credentials.cookie)?;
    let encoded = serde_json::to_string(&credentials)?;
    store::save(ACCOUNT, &encoded)?;
    Ok(())
}

pub fn qq_music() -> Result<Stored<QqMusicCredentials>, CredentialError> {
    match store::read(ACCOUNT)? {
        Stored::Ready(encoded) => {
            let credentials: QqMusicCredentials = serde_json::from_str(&encoded)?;
            validate(&credentials.cookie)?;
            Ok(Stored::Ready(credentials))
        }
        Stored::Missing => Ok(Stored::Missing),
    }
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
