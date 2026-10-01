use vault::Stored;

const ACCOUNT: &str = "qq-music";
const IDENTITY_KEYS: [&str; 4] = ["uin", "qqmusic_uin", "wxuin", "p_uin"];
const PLAYBACK_KEYS: [&str; 4] = ["qm_keyst", "qqmusic_key", "music_key", "wxskey"];

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Credentials {
    pub cookie: String,
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    let stored = vault::read_json(ACCOUNT)?;
    if let Stored::Ready(credentials) = &stored {
        validate(credentials)?;
    }
    Ok(stored)
}

pub fn save(credentials: &Credentials) -> Result<(), vault::Error> {
    validate(credentials)?;
    vault::save_json(ACCOUNT, credentials)
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    let cookie = &credentials.cookie;
    if cookie.trim().is_empty() {
        return Err(vault::Error::Empty("QQ Music cookie"));
    }
    if !has_key(cookie, &IDENTITY_KEYS) {
        return Err(vault::Error::Empty("QQ Music uin cookie"));
    }
    if !has_key(cookie, &PLAYBACK_KEYS) {
        return Err(vault::Error::Empty("QQ Music playback cookie"));
    }
    Ok(())
}

fn has_key(cookie: &str, keys: &[&str]) -> bool {
    cookie
        .split(';')
        .filter_map(|part| part.split_once('='))
        .any(|(key, _)| keys.contains(&key.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(cookie: &str) -> Credentials {
        Credentials {
            cookie: cookie.to_owned(),
        }
    }

    #[test]
    fn requires_identity_and_playback_cookie_fields() {
        assert!(validate(&credentials("")).is_err());
        assert!(validate(&credentials("uin=123")).is_err());
        assert!(validate(&credentials("qm_keyst=secret")).is_err());
        assert!(validate(&credentials("uin=123; qm_keyst=secret")).is_ok());
    }
}
