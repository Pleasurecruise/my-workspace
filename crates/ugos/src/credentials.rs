use serde::{Deserialize, Serialize};
use vault::Stored;

const ACCOUNT: &str = "ugos";
const CERTIFICATE_ACCOUNT: &str = "ugos-certificate";

#[derive(Deserialize, Serialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn read() -> Result<Stored<Credentials>, vault::Error> {
    #[cfg(debug_assertions)]
    {
        let Some([username, password]) =
            vault::variables(["UGOS_USERNAME", "UGOS_PASSWORD"], "UGOS credentials")?
        else {
            return Ok(Stored::Missing);
        };
        let credentials = Credentials { username, password };
        validate(&credentials)?;
        Ok(Stored::Ready(credentials))
    }
    #[cfg(not(debug_assertions))]
    vault::read_json(ACCOUNT)
}

pub fn save(mut credentials: Credentials) -> Result<(), vault::Error> {
    validate(&credentials)?;
    credentials.username = credentials.username.trim().to_owned();
    vault::save_json(ACCOUNT, &credentials)
}

pub fn certificate() -> Result<Stored<String>, vault::Error> {
    vault::read(CERTIFICATE_ACCOUNT)
}

pub fn save_certificate(fingerprint: &str) -> Result<(), vault::Error> {
    if fingerprint.is_empty() {
        return Err(vault::Error::Empty("UGOS certificate fingerprint"));
    }
    vault::save(CERTIFICATE_ACCOUNT, fingerprint)
}

fn validate(credentials: &Credentials) -> Result<(), vault::Error> {
    if credentials.username.trim().is_empty() {
        return Err(vault::Error::Empty("UGOS username"));
    }
    if credentials.password.is_empty() {
        return Err(vault::Error::Empty("UGOS password"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_username() {
        let result = save(Credentials {
            username: String::new(),
            password: "password".to_owned(),
        });
        assert!(matches!(result, Err(vault::Error::Empty("UGOS username"))));
    }
}
