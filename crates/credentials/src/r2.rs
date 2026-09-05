use crate::{CredentialError, Stored, store};
use serde::{Deserialize, Serialize};

const ACCOUNT: &str = "cloudflare-r2";

pub struct R2Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Deserialize, Serialize)]
struct StoredR2Credentials {
    access_key_id: String,
    secret_access_key: String,
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
                    Err(_) => {
                        return Err(CredentialError::InvalidDevelopment("R2 access key ID"));
                    }
                };
                let secret_access_key = match secret_access_key.into_string() {
                    Ok(secret_access_key) => secret_access_key,
                    Err(_) => {
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
        let encoded = match store::read(ACCOUNT)? {
            Stored::Ready(encoded) => encoded,
            Stored::Missing => return Ok(Stored::Missing),
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
    store::save(ACCOUNT, &serde_json::to_string(&stored)?)?;
    Ok(())
}
