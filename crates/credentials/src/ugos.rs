use crate::{CredentialError, SERVICE, Stored};
use serde::{Deserialize, Serialize};

const ACCOUNT: &str = "ugos";
const TLS_PIN_ACCOUNT: &str = "ugos-certificate";

pub struct UgosCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
struct StoredUgosCredentials {
    username: String,
    password: String,
}

pub fn ugos() -> Result<Stored<UgosCredentials>, CredentialError> {
    #[cfg(debug_assertions)]
    {
        let username = std::env::var_os("UGOS_USERNAME");
        let password = std::env::var_os("UGOS_PASSWORD");
        match (username, password) {
            (Some(username), Some(password)) => {
                let username = match username.into_string() {
                    Ok(username) => username,
                    Err(_) => {
                        return Err(CredentialError::InvalidDevelopment("UGOS username"));
                    }
                };
                let password = match password.into_string() {
                    Ok(password) => password,
                    Err(_) => {
                        return Err(CredentialError::InvalidDevelopment("UGOS password"));
                    }
                };
                if username.trim().is_empty() {
                    Err(CredentialError::Empty("UGOS username"))
                } else if password.is_empty() {
                    Err(CredentialError::Empty("UGOS password"))
                } else {
                    Ok(Stored::Ready(UgosCredentials { username, password }))
                }
            }
            (None, None) => Ok(Stored::Missing),
            (Some(..), None) => Err(CredentialError::IncompleteDevelopment("UGOS credentials")),
            (None, Some(..)) => Err(CredentialError::IncompleteDevelopment("UGOS credentials")),
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
        let stored: StoredUgosCredentials = serde_json::from_str(&encoded)?;
        Ok(Stored::Ready(UgosCredentials {
            username: stored.username,
            password: stored.password,
        }))
    }
}

pub fn save_ugos(credentials: UgosCredentials) -> Result<(), CredentialError> {
    if credentials.username.trim().is_empty() {
        return Err(CredentialError::Empty("UGOS username"));
    }
    if credentials.password.is_empty() {
        return Err(CredentialError::Empty("UGOS password"));
    }
    let stored = StoredUgosCredentials {
        username: credentials.username.trim().to_owned(),
        password: credentials.password,
    };
    keyring::Entry::new(SERVICE, ACCOUNT)?.set_password(&serde_json::to_string(&stored)?)?;
    Ok(())
}

pub fn ugos_certificate() -> Result<Stored<String>, CredentialError> {
    let entry = keyring::Entry::new(SERVICE, TLS_PIN_ACCOUNT)?;
    match entry.get_password() {
        Ok(fingerprint) => Ok(Stored::Ready(fingerprint)),
        Err(keyring::Error::NoEntry) => Ok(Stored::Missing),
        Err(error) => Err(CredentialError::Store(error)),
    }
}

pub fn save_ugos_certificate(fingerprint: &str) -> Result<(), CredentialError> {
    if fingerprint.is_empty() {
        return Err(CredentialError::Empty("UGOS certificate fingerprint"));
    }
    keyring::Entry::new(SERVICE, TLS_PIN_ACCOUNT)?.set_password(fingerprint)?;
    Ok(())
}
