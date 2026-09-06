use crate::{CredentialError, Stored, store};
use serde::Deserialize;

const MEMOS_ACCOUNT: &str = "my-memos-api";
const MOMENT_ACCOUNT: &str = "my-moment-api";
const KNOWLEDGE_ACCOUNT: &str = "my-knowledge-api";

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
            Self::Memos => MEMOS_ACCOUNT,
            Self::Moment => MOMENT_ACCOUNT,
            Self::Knowledge => KNOWLEDGE_ACCOUNT,
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
                    Err(_) => return Err(CredentialError::InvalidDevelopment(api.field())),
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
        store::read(api.account())
    }
}

pub fn save_consumer_api(api: ConsumerApi, api_key: &str) -> Result<(), CredentialError> {
    if api_key.trim().is_empty() {
        return Err(CredentialError::Empty(api.field()));
    }
    store::save(api.account(), api_key.trim())?;
    Ok(())
}
