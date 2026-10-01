use serde::Deserialize;
use vault::Stored;

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

    pub(crate) const fn service(self) -> &'static str {
        match self {
            Self::Memos => "my-memos",
            Self::Moment => "my-moment",
            Self::Knowledge => "my-knowledge",
        }
    }

    const fn account(self) -> &'static str {
        match self {
            Self::Memos => "my-memos-api",
            Self::Moment => "my-moment-api",
            Self::Knowledge => "my-knowledge-api",
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

pub fn read(api: ConsumerApi) -> Result<Stored<String>, vault::Error> {
    #[cfg(debug_assertions)]
    {
        let variable = match api {
            ConsumerApi::Memos => "MEMOS_API_KEY",
            ConsumerApi::Moment => "MOMENT_API_KEY",
            ConsumerApi::Knowledge => "KNOWLEDGE_API_KEY",
        };
        let Some([api_key]) = vault::variables([variable], api.field())? else {
            return Ok(Stored::Missing);
        };
        if api_key.trim().is_empty() {
            return Err(vault::Error::Empty(api.field()));
        }
        Ok(Stored::Ready(api_key))
    }
    #[cfg(not(debug_assertions))]
    vault::read(api.account())
}

pub fn save(api: ConsumerApi, api_key: &str) -> Result<(), vault::Error> {
    if api_key.trim().is_empty() {
        return Err(vault::Error::Empty(api.field()));
    }
    vault::save(api.account(), api_key.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_api_key() {
        assert!(matches!(
            save(ConsumerApi::Memos, "  "),
            Err(vault::Error::Empty("my-memos API key"))
        ));
    }
}
