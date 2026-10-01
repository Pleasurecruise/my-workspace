use vault::Stored;

const ACCOUNT: &str = "spotify-music";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Credentials {
    pub web_client_id: Option<String>,
    pub web_refresh_token: String,
    pub playback_refresh_token: String,
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
    if credentials.web_refresh_token.trim().is_empty() {
        return Err(vault::Error::Empty("Spotify Web refresh token"));
    }
    if credentials.playback_refresh_token.trim().is_empty() {
        return Err(vault::Error::Empty("Spotify playback refresh token"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(web: &str, playback: &str) -> Credentials {
        Credentials {
            web_client_id: None,
            web_refresh_token: web.to_owned(),
            playback_refresh_token: playback.to_owned(),
        }
    }

    #[test]
    fn rejects_incomplete_credentials() {
        assert!(validate(&credentials("", "playback")).is_err());
        assert!(validate(&credentials("web", "")).is_err());
        assert!(validate(&credentials("web", "playback")).is_ok());
    }
}
