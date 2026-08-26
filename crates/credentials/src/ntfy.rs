use crate::{CredentialError, SERVICE, Stored};
use serde::{Deserialize, Serialize};

const ACCOUNT: &str = "ntfy-notifications";

#[derive(Clone, Deserialize, Serialize)]
pub struct NtfyConfig {
    #[serde(default)]
    pub token: String,
    pub development: bool,
}

pub fn ntfy() -> Result<Stored<NtfyConfig>, CredentialError> {
    #[cfg(debug_assertions)]
    {
        match std::env::var_os("NTFY_TOKEN") {
            None => Ok(Stored::Missing),
            Some(token) => {
                let configuration = NtfyConfig {
                    token: token
                        .into_string()
                        .map_err(|_| CredentialError::InvalidDevelopment("ntfy token"))?,
                    development: true,
                };
                validate(&configuration)?;
                Ok(Stored::Ready(configuration))
            }
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        let encoded = match entry.get_password() {
            Ok(encoded) => encoded,
            Err(keyring::Error::NoEntry) => return Ok(Stored::Missing),
            Err(error) => return Err(CredentialError::Store(error)),
        };
        let mut configuration: NtfyConfig = serde_json::from_str(&encoded)?;
        if configuration.token.trim().is_empty() {
            return Ok(Stored::Missing);
        }
        configuration.development = false;
        validate(&configuration)?;
        Ok(Stored::Ready(configuration))
    }
}

pub fn save_ntfy(mut configuration: NtfyConfig) -> Result<(), CredentialError> {
    validate(&configuration)?;
    configuration.token = configuration.token.trim().to_owned();
    configuration.development = false;
    keyring::Entry::new(SERVICE, ACCOUNT)?.set_password(&serde_json::to_string(&configuration)?)?;
    Ok(())
}

fn validate(configuration: &NtfyConfig) -> Result<(), CredentialError> {
    if configuration.token.trim().is_empty() {
        return Err(CredentialError::Empty("ntfy token"));
    }
    Ok(())
}
