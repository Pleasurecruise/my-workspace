use crate::{CredentialError, Stored, store};

const ACCOUNT: &str = "app-lock";

pub struct AppLock {
    pub password: String,
    pub development: bool,
}

pub fn app_lock() -> Result<Stored<AppLock>, CredentialError> {
    #[cfg(debug_assertions)]
    if let Some(password) = std::env::var_os("APP_LOCK_PASSWORD") {
        let password = password
            .into_string()
            .map_err(|_| CredentialError::InvalidDevelopment("app lock password"))?;
        validate(&password)?;
        return Ok(Stored::Ready(AppLock {
            password,
            development: true,
        }));
    }
    match store::read(ACCOUNT)? {
        Stored::Ready(password) => Ok(Stored::Ready(AppLock {
            password,
            development: false,
        })),
        Stored::Missing => Ok(Stored::Missing),
    }
}

pub fn save_app_lock(password: &str) -> Result<(), CredentialError> {
    validate(password)?;
    store::save(ACCOUNT, password)?;
    Ok(())
}

pub fn delete_app_lock() -> Result<(), CredentialError> {
    store::delete(ACCOUNT)
}

fn validate(password: &str) -> Result<(), CredentialError> {
    if password.trim().is_empty() {
        return Err(CredentialError::Empty("app lock password"));
    }
    if password.chars().count() < 4 {
        return Err(CredentialError::TooShort("app lock password", 4));
    }
    Ok(())
}
