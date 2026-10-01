use serde::{Deserialize, Serialize};
use vault::Stored;

const ACCOUNT: &str = "telegram-publication";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub api_id: i32,
    pub api_hash: String,
    pub channel_username: String,
}

impl Credentials {
    fn normalize(mut self) -> Self {
        self.api_hash = self.api_hash.trim().to_owned();
        self.channel_username = self
            .channel_username
            .trim()
            .trim_start_matches('@')
            .to_owned();
        self
    }
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    #[cfg(debug_assertions)]
    if let Some([api_id, api_hash, channel_username]) = vault::variables(
        [
            "TELEGRAM_API_ID",
            "TELEGRAM_API_HASH",
            "TELEGRAM_CHANNEL_USERNAME",
        ],
        "Telegram publication credentials",
    )? {
        let api_id = api_id.parse().map_err(|_| {
            vault::Error::InvalidValue("Telegram API ID", "must be a positive integer")
        })?;
        let credentials = Credentials {
            api_id,
            api_hash,
            channel_username,
        }
        .normalize();
        validate(&credentials)?;
        return Ok(Stored::Ready(credentials));
    }
    let stored = vault::read_json(ACCOUNT)?;
    if let Stored::Ready(credentials) = &stored {
        validate(credentials)?;
    }
    Ok(stored)
}

pub fn save(credentials: Credentials) -> Result<(), vault::Error> {
    let credentials = credentials.normalize();
    validate(&credentials)?;
    vault::save_json(ACCOUNT, &credentials)
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    if credentials.api_id <= 0 {
        return Err(vault::Error::InvalidValue(
            "Telegram API ID",
            "must be a positive integer",
        ));
    }
    let api_hash = credentials.api_hash.trim();
    if api_hash.is_empty() {
        return Err(vault::Error::Empty("Telegram API hash"));
    }
    if api_hash.len() != 32
        || !api_hash
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(vault::Error::InvalidValue(
            "Telegram API hash",
            "must contain 32 hexadecimal characters",
        ));
    }
    let channel = credentials.channel_username.trim().trim_start_matches('@');
    if channel.is_empty() {
        return Err(vault::Error::Empty("Telegram channel username"));
    }
    let valid_channel = (5..=32).contains(&channel.len())
        && channel
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if !valid_channel {
        return Err(vault::Error::InvalidValue(
            "Telegram channel username",
            "must be a 5-32 character public username",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(channel_username: &str) -> Credentials {
        Credentials {
            api_id: 12345,
            api_hash: "0123456789abcdef0123456789abcdef".to_owned(),
            channel_username: channel_username.to_owned(),
        }
    }

    #[test]
    fn rejects_empty_channel() {
        assert!(matches!(
            save(credentials("  ")),
            Err(vault::Error::Empty("Telegram channel username"))
        ));
    }

    #[test]
    fn rejects_invalid_channel() {
        assert!(matches!(
            save(credentials("@my channel")),
            Err(vault::Error::InvalidValue("Telegram channel username", _))
        ));
    }
}
