use crate::{CredentialError, SERVICE, Stored};
use serde::{Deserialize, Serialize};

const TELEGRAM_ACCOUNT: &str = "telegram-publication";
const X_ACCOUNT: &str = "x-publication";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramCredentials {
    pub api_id: i32,
    pub api_hash: String,
    pub channel_username: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XCredentials {
    pub client_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

pub fn telegram() -> Result<Stored<TelegramCredentials>, CredentialError> {
    #[cfg(debug_assertions)]
    {
        let api_id = std::env::var_os("TELEGRAM_API_ID");
        let api_hash = std::env::var_os("TELEGRAM_API_HASH");
        let channel_username = std::env::var_os("TELEGRAM_CHANNEL_USERNAME");
        match (api_id, api_hash, channel_username) {
            (None, None, None) => read_json(TELEGRAM_ACCOUNT, validate_telegram),
            (Some(api_id), Some(api_hash), Some(channel_username)) => {
                let mut credentials = TelegramCredentials {
                    api_id: api_id
                        .into_string()
                        .map_err(|_| CredentialError::InvalidDevelopment("Telegram API ID"))?
                        .parse()
                        .map_err(|_| {
                            CredentialError::InvalidValue(
                                "Telegram API ID",
                                "must be a positive integer",
                            )
                        })?,
                    api_hash: api_hash
                        .into_string()
                        .map_err(|_| CredentialError::InvalidDevelopment("Telegram API hash"))?,
                    channel_username: channel_username.into_string().map_err(|_| {
                        CredentialError::InvalidDevelopment("Telegram channel username")
                    })?,
                };
                credentials.api_hash = credentials.api_hash.trim().to_owned();
                credentials.channel_username = credentials
                    .channel_username
                    .trim()
                    .trim_start_matches('@')
                    .to_owned();
                validate_telegram(&credentials)?;
                Ok(Stored::Ready(credentials))
            }
            _ => Err(CredentialError::IncompleteDevelopment(
                "Telegram publication credentials",
            )),
        }
    }
    #[cfg(not(debug_assertions))]
    {
        read_json(TELEGRAM_ACCOUNT, validate_telegram)
    }
}

pub fn save_telegram(mut credentials: TelegramCredentials) -> Result<(), CredentialError> {
    credentials.api_hash = credentials.api_hash.trim().to_owned();
    credentials.channel_username = credentials
        .channel_username
        .trim()
        .trim_start_matches('@')
        .to_owned();
    validate_telegram(&credentials)?;
    save_json(TELEGRAM_ACCOUNT, &credentials)
}

pub fn x() -> Result<Stored<XCredentials>, CredentialError> {
    read_json(X_ACCOUNT, validate_x)
}

pub fn save_x(mut credentials: XCredentials) -> Result<(), CredentialError> {
    credentials.client_id = credentials.client_id.trim().to_owned();
    credentials.access_token = credentials.access_token.trim().to_owned();
    credentials.refresh_token = credentials.refresh_token.trim().to_owned();
    validate_x(&credentials)?;
    save_json(X_ACCOUNT, &credentials)
}

fn read_json<T>(
    account: &'static str,
    validate: fn(&T) -> Result<(), CredentialError>,
) -> Result<Stored<T>, CredentialError>
where
    T: serde::de::DeserializeOwned,
{
    let entry = keyring::Entry::new(SERVICE, account)?;
    let encoded = match entry.get_password() {
        Ok(encoded) => encoded,
        Err(keyring::Error::NoEntry) => return Ok(Stored::Missing),
        Err(error) => return Err(CredentialError::Store(error)),
    };
    let credentials = serde_json::from_str(&encoded)?;
    validate(&credentials)?;
    Ok(Stored::Ready(credentials))
}

fn save_json<T: Serialize>(account: &'static str, value: &T) -> Result<(), CredentialError> {
    keyring::Entry::new(SERVICE, account)?.set_password(&serde_json::to_string(value)?)?;
    Ok(())
}

fn validate_telegram(credentials: &TelegramCredentials) -> Result<(), CredentialError> {
    if credentials.api_id <= 0 {
        return Err(CredentialError::InvalidValue(
            "Telegram API ID",
            "must be a positive integer",
        ));
    }
    let api_hash = credentials.api_hash.trim();
    if api_hash.is_empty() {
        return Err(CredentialError::Empty("Telegram API hash"));
    }
    if api_hash.len() != 32
        || !api_hash
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(CredentialError::InvalidValue(
            "Telegram API hash",
            "must contain 32 hexadecimal characters",
        ));
    }
    let channel = credentials.channel_username.trim().trim_start_matches('@');
    if channel.is_empty() {
        return Err(CredentialError::Empty("Telegram channel username"));
    }
    if !(5..=32).contains(&channel.len())
        || !channel
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(CredentialError::InvalidValue(
            "Telegram channel username",
            "must be a 5-32 character public username",
        ));
    }
    Ok(())
}

fn validate_x(credentials: &XCredentials) -> Result<(), CredentialError> {
    if credentials.client_id.trim().is_empty() {
        return Err(CredentialError::Empty("X OAuth client ID"));
    }
    if credentials.access_token.trim().is_empty() {
        return Err(CredentialError::Empty("X OAuth access token"));
    }
    if credentials.refresh_token.trim().is_empty() {
        return Err(CredentialError::Empty("X OAuth refresh token"));
    }
    if credentials.expires_at == 0 {
        return Err(CredentialError::InvalidValue(
            "X OAuth expiration",
            "must be a Unix timestamp",
        ));
    }
    Ok(())
}
