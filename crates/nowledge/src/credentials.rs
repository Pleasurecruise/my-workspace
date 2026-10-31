use serde::{Deserialize, Serialize};
use vault::Stored;

const ACCOUNT: &str = "nowledge-mem";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub url: String,
    pub api_key: String,
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    #[cfg(debug_assertions)]
    {
        let Some([url, api_key]) = vault::variables(
            ["NOWLEDGE_API_URL", "NOWLEDGE_API_KEY"],
            "Nowledge Mem credentials",
        )?
        else {
            return Ok(Stored::Missing);
        };
        let credentials = Credentials { url, api_key };
        validate(&credentials)?;
        Ok(Stored::Ready(credentials))
    }
    #[cfg(not(debug_assertions))]
    {
        let stored = vault::read_json(ACCOUNT)?;
        if let Stored::Ready(credentials) = &stored {
            validate(credentials)?;
        }
        Ok(stored)
    }
}

pub fn save(credentials: Credentials) -> Result<(), vault::Error> {
    validate(&credentials)?;
    vault::save_json(
        ACCOUNT,
        &Credentials {
            url: credentials.url.trim().trim_end_matches('/').to_owned(),
            api_key: credentials.api_key.trim().to_owned(),
        },
    )
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    let url = credentials.url.trim();
    if url.is_empty() {
        return Err(vault::Error::Empty("Nowledge Mem server URL"));
    }
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(vault::Error::InvalidValue(
            "Nowledge Mem server URL",
            "use an http or https address",
        ));
    }
    if credentials.api_key.trim().is_empty() {
        return Err(vault::Error::Empty("Nowledge Mem API key"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_credentials() {
        let credentials = |url: &str, api_key: &str| Credentials {
            url: url.to_owned(),
            api_key: api_key.to_owned(),
        };
        assert!(matches!(
            save(credentials(" ", "key")),
            Err(vault::Error::Empty("Nowledge Mem server URL"))
        ));
        assert!(matches!(
            save(credentials("nmem.example.com", "key")),
            Err(vault::Error::InvalidValue("Nowledge Mem server URL", _))
        ));
        assert!(matches!(
            save(credentials("https://nmem.example.com", " ")),
            Err(vault::Error::Empty("Nowledge Mem API key"))
        ));
    }
}
