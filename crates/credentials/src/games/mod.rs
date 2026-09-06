use crate::{CredentialError, Stored};
pub mod accounts;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct RecordDevice {
    pub id: String,
    pub fingerprint: String,
    pub updated_at: i64,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "provider", rename_all = "camelCase")]
pub enum Session {
    Mihoyo {
        account_id: String,
        stoken: String,
        mid: String,
        ltoken: String,
        device: String,
        #[serde(default)]
        record: Option<RecordDevice>,
    },
    Skland {
        token: String,
        cred: String,
        sign_token: String,
        user_id: String,
    },
    Steam {
        api_key: String,
        steam_id: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    Mihoyo,
    Skland,
    Steam,
}

impl Provider {
    pub fn key(self) -> &'static str {
        match self {
            Self::Mihoyo => "games-mihoyo",
            Self::Skland => "games-skland",
            Self::Steam => "games-steam",
        }
    }
}

impl Session {
    pub fn provider(&self) -> Provider {
        match self {
            Self::Mihoyo { .. } => Provider::Mihoyo,
            Self::Skland { .. } => Provider::Skland,
            Self::Steam { .. } => Provider::Steam,
        }
    }

    fn validate(&self) -> Result<(), CredentialError> {
        let fields: &[&str] = match self {
            Self::Mihoyo {
                account_id,
                stoken,
                mid,
                ltoken,
                device,
                ..
            } => &[account_id, stoken, mid, ltoken, device],
            Self::Skland {
                token,
                cred,
                sign_token,
                user_id,
            } => &[token, cred, sign_token, user_id],
            Self::Steam { api_key, steam_id } => {
                if steam_id.len() != 17 || !steam_id.bytes().all(|c| c.is_ascii_digit()) {
                    return Err(CredentialError::InvalidValue(
                        "SteamID",
                        "expected a 17-digit SteamID64",
                    ));
                }
                &[api_key, steam_id]
            }
        };
        if fields
            .iter()
            .any(|field| field.trim().is_empty() || field.contains(['\r', '\n', ';']))
        {
            return Err(CredentialError::InvalidValue(
                "game session",
                "missing or invalid session field",
            ));
        }
        let Self::Mihoyo {
            record: Some(record),
            ..
        } = self
        else {
            return Ok(());
        };
        if [&record.id, &record.fingerprint]
            .iter()
            .any(|field| field.trim().is_empty() || field.contains(['\r', '\n', ';']))
        {
            return Err(CredentialError::InvalidValue(
                "game record device",
                "invalid device field",
            ));
        }
        Ok(())
    }
}

pub fn read(provider: Provider) -> Result<Stored<Session>, CredentialError> {
    #[cfg(debug_assertions)]
    if provider == Provider::Steam {
        match (
            std::env::var_os("STEAM_API_KEY"),
            std::env::var_os("STEAM_ID"),
        ) {
            (Some(api_key), Some(steam_id)) => {
                let session = Session::Steam {
                    api_key: api_key
                        .into_string()
                        .map_err(|_| CredentialError::InvalidDevelopment("Steam"))?,
                    steam_id: steam_id
                        .into_string()
                        .map_err(|_| CredentialError::InvalidDevelopment("Steam"))?,
                };
                session.validate()?;
                return Ok(Stored::Ready(session));
            }
            (None, None) => {}
            _ => return Err(CredentialError::IncompleteDevelopment("Steam")),
        }
    }
    if provider == Provider::Mihoyo {
        let accounts = accounts::read()?;
        return match accounts.sessions.len() {
            0 => Ok(Stored::Missing),
            1 => Ok(accounts
                .sessions
                .into_values()
                .next()
                .map_or(Stored::Missing, Stored::Ready)),
            _ => Err(CredentialError::InvalidValue(
                "miHoYo account",
                "select an account for this game",
            )),
        };
    }
    let stored = match crate::store::read(provider.key())? {
        Stored::Missing => Stored::Missing,
        Stored::Ready(encoded) => Stored::Ready(serde_json::from_str::<Session>(&encoded)?),
    };
    if let Stored::Ready(session) = &stored {
        session.validate()?;
        if session.provider() != provider {
            return Err(CredentialError::InvalidStore);
        }
    }
    Ok(stored)
}

pub fn save(session: &Session) -> Result<(), CredentialError> {
    session.validate()?;
    if session.provider() == Provider::Mihoyo {
        return accounts::update(|accounts| accounts.insert(session.clone()));
    }
    crate::store::save(session.provider().key(), &serde_json::to_string(session)?)
}

#[cfg(test)]
#[path = "../../tests/unit/games.rs"]
mod tests;
