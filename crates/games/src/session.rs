use vault::Stored;

pub mod mihoyo;

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

    fn validate(&self) -> Result<(), vault::Error> {
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
                    return Err(vault::Error::InvalidValue(
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
            return Err(vault::Error::InvalidValue(
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
            return Err(vault::Error::InvalidValue(
                "game record device",
                "invalid device field",
            ));
        }
        Ok(())
    }
}

pub fn read(provider: Provider) -> Result<Stored<Session>, vault::Error> {
    #[cfg(debug_assertions)]
    if provider == Provider::Steam
        && let Some([api_key, steam_id]) = vault::variables(["STEAM_API_KEY", "STEAM_ID"], "Steam")?
    {
        let session = Session::Steam { api_key, steam_id };
        session.validate()?;
        return Ok(Stored::Ready(session));
    }
    if provider == Provider::Mihoyo {
        let accounts = mihoyo::read()?;
        return match accounts.sessions.len() {
            0 => Ok(Stored::Missing),
            1 => Ok(accounts
                .sessions
                .into_values()
                .next()
                .map_or(Stored::Missing, Stored::Ready)),
            _ => Err(vault::Error::InvalidValue(
                "miHoYo account",
                "select an account for this game",
            )),
        };
    }
    let stored = vault::read_json::<Session>(provider.key())?;
    if let Stored::Ready(session) = &stored {
        session.validate()?;
        if session.provider() != provider {
            return Err(vault::Error::InvalidStore);
        }
    }
    Ok(stored)
}

pub fn save(session: &Session) -> Result<(), vault::Error> {
    session.validate()?;
    if session.provider() == Provider::Mihoyo {
        return mihoyo::update(|accounts| accounts.insert(session.clone()));
    }
    vault::save_json(session.provider().key(), session)
}

#[cfg(test)]
#[path = "../tests/unit/session.rs"]
mod tests;
