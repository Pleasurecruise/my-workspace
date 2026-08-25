use serde::{Deserialize, Serialize};
#[cfg(debug_assertions)]
use std::path::PathBuf;

const SERVICE: &str = "me.you-find.vesper";
const UGOS_ACCOUNT: &str = "ugos";
const UGOS_TLS_PIN_ACCOUNT: &str = "ugos-certificate";
const R2_ACCOUNT: &str = "cloudflare-r2";
const MEMOS_API_ACCOUNT: &str = "my-memos-api";
const MOMENT_API_ACCOUNT: &str = "my-moment-api";
const KNOWLEDGE_API_ACCOUNT: &str = "my-knowledge-api";
const NTFY_NOTIFICATIONS_ACCOUNT: &str = "ntfy-notifications";
const APP_LOCK_ACCOUNT: &str = "app-lock";

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsumerApi {
    Memos,
    Moment,
    Knowledge,
}

impl ConsumerApi {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Memos => "memos",
            Self::Moment => "moment",
            Self::Knowledge => "knowledge",
        }
    }

    const fn account(self) -> &'static str {
        match self {
            Self::Memos => MEMOS_API_ACCOUNT,
            Self::Moment => MOMENT_API_ACCOUNT,
            Self::Knowledge => KNOWLEDGE_API_ACCOUNT,
        }
    }

    const fn field(self) -> &'static str {
        match self {
            Self::Memos => "my-memos API key",
            Self::Moment => "my-moment API key",
            Self::Knowledge => "my-knowledge API key",
        }
    }
}

pub struct UgosCredentials {
    pub username: String,
    pub password: String,
}

pub struct R2Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub struct AppLock {
    pub password: String,
    pub development: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct NtfyConfig {
    #[serde(default)]
    pub token: String,
    pub development: bool,
}

pub enum Stored<T> {
    Missing,
    Ready(T),
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("system credential store failed: {0}")]
    Store(#[from] keyring::Error),
    #[error("stored credential is invalid: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("credential field {0} cannot be empty")]
    Empty(&'static str),
    #[error("credential field {0} must contain at least {1} characters")]
    TooShort(&'static str, usize),
    #[error("development credential {0} is incomplete")]
    IncompleteDevelopment(&'static str),
    #[error("development credential {0} is not valid Unicode")]
    InvalidDevelopment(&'static str),
    #[cfg(debug_assertions)]
    #[error("could not load development credentials from {}: {source}", path.display())]
    DevelopmentFile {
        path: PathBuf,
        source: dotenvy::Error,
    },
}

#[cfg(debug_assertions)]
pub fn load_development_environment() -> Result<(), CredentialError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    if !path.exists() {
        return Ok(());
    }
    match dotenvy::from_path(&path) {
        Ok(()) => Ok(()),
        Err(source) => Err(CredentialError::DevelopmentFile { path, source }),
    }
}

#[derive(Deserialize, Serialize)]
struct StoredUgosCredentials {
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct StoredR2Credentials {
    access_key_id: String,
    secret_access_key: String,
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
                    Err(..) => {
                        return Err(CredentialError::InvalidDevelopment("UGOS username"));
                    }
                };
                let password = match password.into_string() {
                    Ok(password) => password,
                    Err(..) => {
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
        let entry = keyring::Entry::new(SERVICE, UGOS_ACCOUNT)?;
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
    keyring::Entry::new(SERVICE, UGOS_ACCOUNT)?.set_password(&serde_json::to_string(&stored)?)?;
    Ok(())
}

pub fn ugos_certificate() -> Result<Stored<String>, CredentialError> {
    let entry = keyring::Entry::new(SERVICE, UGOS_TLS_PIN_ACCOUNT)?;
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
    keyring::Entry::new(SERVICE, UGOS_TLS_PIN_ACCOUNT)?.set_password(fingerprint)?;
    Ok(())
}

pub fn r2() -> Result<Stored<R2Credentials>, CredentialError> {
    #[cfg(debug_assertions)]
    {
        let access_key_id = std::env::var_os("R2_ACCESS_KEY_ID");
        let secret_access_key = std::env::var_os("R2_SECRET_ACCESS_KEY");
        match (access_key_id, secret_access_key) {
            (Some(access_key_id), Some(secret_access_key)) => {
                let access_key_id = match access_key_id.into_string() {
                    Ok(access_key_id) => access_key_id,
                    Err(..) => {
                        return Err(CredentialError::InvalidDevelopment("R2 access key ID"));
                    }
                };
                let secret_access_key = match secret_access_key.into_string() {
                    Ok(secret_access_key) => secret_access_key,
                    Err(..) => {
                        return Err(CredentialError::InvalidDevelopment("R2 secret access key"));
                    }
                };
                if access_key_id.trim().is_empty() {
                    Err(CredentialError::Empty("R2 access key ID"))
                } else if secret_access_key.is_empty() {
                    Err(CredentialError::Empty("R2 secret access key"))
                } else {
                    Ok(Stored::Ready(R2Credentials {
                        access_key_id,
                        secret_access_key,
                    }))
                }
            }
            (None, None) => Ok(Stored::Missing),
            (Some(..), None) => Err(CredentialError::IncompleteDevelopment("R2 credentials")),
            (None, Some(..)) => Err(CredentialError::IncompleteDevelopment("R2 credentials")),
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let entry = keyring::Entry::new(SERVICE, R2_ACCOUNT)?;
        let encoded = match entry.get_password() {
            Ok(encoded) => encoded,
            Err(keyring::Error::NoEntry) => return Ok(Stored::Missing),
            Err(error) => return Err(CredentialError::Store(error)),
        };
        let stored: StoredR2Credentials = serde_json::from_str(&encoded)?;
        Ok(Stored::Ready(R2Credentials {
            access_key_id: stored.access_key_id,
            secret_access_key: stored.secret_access_key,
        }))
    }
}

pub fn save_r2(credentials: R2Credentials) -> Result<(), CredentialError> {
    if credentials.access_key_id.trim().is_empty() {
        return Err(CredentialError::Empty("R2 access key ID"));
    }
    if credentials.secret_access_key.is_empty() {
        return Err(CredentialError::Empty("R2 secret access key"));
    }
    let stored = StoredR2Credentials {
        access_key_id: credentials.access_key_id.trim().to_owned(),
        secret_access_key: credentials.secret_access_key,
    };
    keyring::Entry::new(SERVICE, R2_ACCOUNT)?.set_password(&serde_json::to_string(&stored)?)?;
    Ok(())
}

pub fn consumer_api(api: ConsumerApi) -> Result<Stored<String>, CredentialError> {
    #[cfg(debug_assertions)]
    {
        let variable = match api {
            ConsumerApi::Memos => "MEMOS_API_KEY",
            ConsumerApi::Moment => "MOMENT_API_KEY",
            ConsumerApi::Knowledge => "KNOWLEDGE_API_KEY",
        };
        match std::env::var_os(variable) {
            Some(api_key) => {
                let api_key = match api_key.into_string() {
                    Ok(api_key) => api_key,
                    Err(..) => return Err(CredentialError::InvalidDevelopment(api.field())),
                };
                if api_key.trim().is_empty() {
                    Err(CredentialError::Empty(api.field()))
                } else {
                    Ok(Stored::Ready(api_key))
                }
            }
            None => Ok(Stored::Missing),
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let entry = keyring::Entry::new(SERVICE, api.account())?;
        match entry.get_password() {
            Ok(api_key) => Ok(Stored::Ready(api_key)),
            Err(keyring::Error::NoEntry) => Ok(Stored::Missing),
            Err(error) => Err(CredentialError::Store(error)),
        }
    }
}

pub fn save_consumer_api(api: ConsumerApi, api_key: &str) -> Result<(), CredentialError> {
    if api_key.trim().is_empty() {
        return Err(CredentialError::Empty(api.field()));
    }
    keyring::Entry::new(SERVICE, api.account())?.set_password(api_key.trim())?;
    Ok(())
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
                validate_ntfy(&configuration)?;
                Ok(Stored::Ready(configuration))
            }
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let entry = keyring::Entry::new(SERVICE, NTFY_NOTIFICATIONS_ACCOUNT)?;
        let encoded = match entry.get_password() {
            Ok(encoded) => encoded,
            Err(keyring::Error::NoEntry) => return Ok(Stored::Missing),
            Err(error) => return Err(CredentialError::Store(error)),
        };
        let mut configuration: NtfyConfig = match serde_json::from_str(&encoded) {
            Ok(configuration) => configuration,
            Err(_) => return Ok(Stored::Missing),
        };
        if configuration.token.trim().is_empty() {
            return Ok(Stored::Missing);
        }
        configuration.development = false;
        validate_ntfy(&configuration)?;
        Ok(Stored::Ready(configuration))
    }
}

pub fn save_ntfy(mut configuration: NtfyConfig) -> Result<(), CredentialError> {
    validate_ntfy(&configuration)?;
    configuration.token = configuration.token.trim().to_owned();
    configuration.development = false;
    keyring::Entry::new(SERVICE, NTFY_NOTIFICATIONS_ACCOUNT)?
        .set_password(&serde_json::to_string(&configuration)?)?;
    Ok(())
}

fn validate_ntfy(configuration: &NtfyConfig) -> Result<(), CredentialError> {
    if configuration.token.trim().is_empty() {
        return Err(CredentialError::Empty("ntfy token"));
    }
    Ok(())
}

pub fn app_lock() -> Result<Stored<AppLock>, CredentialError> {
    let entry = keyring::Entry::new(SERVICE, APP_LOCK_ACCOUNT)?;
    match entry.get_password() {
        Ok(password) => Ok(Stored::Ready(AppLock {
            password,
            development: false,
        })),
        Err(keyring::Error::NoEntry) => {
            #[cfg(debug_assertions)]
            if let Some(password) = std::env::var_os("APP_LOCK_PASSWORD") {
                let password = password
                    .into_string()
                    .map_err(|_| CredentialError::InvalidDevelopment("app lock password"))?;
                validate_app_lock(&password)?;
                return Ok(Stored::Ready(AppLock {
                    password,
                    development: true,
                }));
            }
            Ok(Stored::Missing)
        }
        Err(error) => Err(CredentialError::Store(error)),
    }
}

pub fn save_app_lock(password: &str) -> Result<(), CredentialError> {
    validate_app_lock(password)?;
    keyring::Entry::new(SERVICE, APP_LOCK_ACCOUNT)?.set_password(password)?;
    Ok(())
}

fn validate_app_lock(password: &str) -> Result<(), CredentialError> {
    if password.trim().is_empty() {
        return Err(CredentialError::Empty("app lock password"));
    }
    if password.chars().count() < 4 {
        return Err(CredentialError::TooShort("app lock password", 4));
    }
    Ok(())
}

pub fn delete_app_lock() -> Result<(), CredentialError> {
    let entry = keyring::Entry::new(SERVICE, APP_LOCK_ACCOUNT)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(CredentialError::Store(error)),
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
