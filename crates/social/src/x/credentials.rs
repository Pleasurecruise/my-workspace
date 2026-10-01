use serde::{Deserialize, Serialize};
use vault::Stored;

const ACCOUNT: &str = "x-publication";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub client_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    let stored = vault::read_json(ACCOUNT)?;
    if let Stored::Ready(credentials) = &stored {
        validate(credentials)?;
    }
    Ok(stored)
}

pub fn save(mut credentials: Credentials) -> Result<(), vault::Error> {
    credentials.client_id = credentials.client_id.trim().to_owned();
    credentials.access_token = credentials.access_token.trim().to_owned();
    credentials.refresh_token = credentials.refresh_token.trim().to_owned();
    validate(&credentials)?;
    vault::save_json(ACCOUNT, &credentials)
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    if credentials.client_id.trim().is_empty() {
        return Err(vault::Error::Empty("X OAuth client ID"));
    }
    if credentials.access_token.trim().is_empty() {
        return Err(vault::Error::Empty("X OAuth access token"));
    }
    if credentials.refresh_token.trim().is_empty() {
        return Err(vault::Error::Empty("X OAuth refresh token"));
    }
    if credentials.expires_at == 0 {
        return Err(vault::Error::InvalidValue(
            "X OAuth expiration",
            "must be a Unix timestamp",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_access_token() {
        let result = save(Credentials {
            client_id: "client".to_owned(),
            access_token: "  ".to_owned(),
            refresh_token: "refresh".to_owned(),
            expires_at: 1,
        });
        assert!(matches!(
            result,
            Err(vault::Error::Empty("X OAuth access token"))
        ));
    }
}
