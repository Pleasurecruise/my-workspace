use crate::{CredentialError, Stored, store};
use serde::{Deserialize, Serialize};

const ACCOUNT: &str = "codex-resets";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexResets {
    pub enabled: bool,
}

pub fn codex_resets() -> Result<Stored<CodexResets>, CredentialError> {
    match store::read(ACCOUNT)? {
        Stored::Missing => Ok(Stored::Missing),
        Stored::Ready(value) => Ok(Stored::Ready(serde_json::from_str(&value)?)),
    }
}

pub fn save_codex_resets(configuration: CodexResets) -> Result<(), CredentialError> {
    if configuration.enabled {
        store::save(ACCOUNT, &serde_json::to_string(&configuration)?)
    } else {
        store::delete(ACCOUNT)
    }
}
