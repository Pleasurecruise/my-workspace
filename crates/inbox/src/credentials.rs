use serde::{Deserialize, Serialize};
use vault::Stored;

const ACCOUNT: &str = "ntfy-notifications";

#[derive(Clone, Deserialize, Serialize)]
pub struct Credentials {
    pub token: String,
    pub development: bool,
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    #[cfg(debug_assertions)]
    {
        let Some([token]) = vault::variables(["NTFY_TOKEN"], "ntfy token")? else {
            return Ok(Stored::Missing);
        };
        let credentials = Credentials {
            token,
            development: true,
        };
        validate(&credentials)?;
        Ok(Stored::Ready(credentials))
    }
    #[cfg(not(debug_assertions))]
    {
        let Stored::Ready(mut credentials) = vault::read_json::<Credentials>(ACCOUNT)? else {
            return Ok(Stored::Missing);
        };
        credentials.development = false;
        validate(&credentials)?;
        Ok(Stored::Ready(credentials))
    }
}

pub fn save(mut credentials: Credentials) -> Result<(), vault::Error> {
    validate(&credentials)?;
    credentials.token = credentials.token.trim().to_owned();
    credentials.development = false;
    vault::save_json(ACCOUNT, &credentials)
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    if credentials.token.trim().is_empty() {
        return Err(vault::Error::Empty("ntfy token"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_token() {
        let result = save(Credentials {
            token: "  ".to_owned(),
            development: false,
        });
        assert!(matches!(result, Err(vault::Error::Empty("ntfy token"))));
    }
}
