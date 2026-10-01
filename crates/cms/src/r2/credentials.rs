use serde::{Deserialize, Serialize};
use vault::Stored;

const ACCOUNT: &str = "cloudflare-r2";

#[derive(Deserialize, Serialize)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    #[cfg(debug_assertions)]
    {
        let Some([access_key_id, secret_access_key]) = vault::variables(
            ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"],
            "R2 credentials",
        )?
        else {
            return Ok(Stored::Missing);
        };
        let credentials = Credentials {
            access_key_id,
            secret_access_key,
        };
        validate(&credentials)?;
        Ok(Stored::Ready(credentials))
    }
    #[cfg(not(debug_assertions))]
    vault::read_json(ACCOUNT)
}

pub fn save(mut credentials: Credentials) -> Result<(), vault::Error> {
    validate(&credentials)?;
    credentials.access_key_id = credentials.access_key_id.trim().to_owned();
    vault::save_json(ACCOUNT, &credentials)
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    if credentials.access_key_id.trim().is_empty() {
        return Err(vault::Error::Empty("R2 access key ID"));
    }
    if credentials.secret_access_key.is_empty() {
        return Err(vault::Error::Empty("R2 secret access key"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_secret() {
        let result = save(Credentials {
            access_key_id: "access".to_owned(),
            secret_access_key: String::new(),
        });
        assert!(matches!(
            result,
            Err(vault::Error::Empty("R2 secret access key"))
        ));
    }
}
