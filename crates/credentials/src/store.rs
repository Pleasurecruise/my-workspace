use crate::{CredentialError, SERVICE, Stored};

#[cfg(target_os = "macos")]
mod macos;

pub(crate) fn read(account: &str) -> Result<Stored<String>, CredentialError> {
    #[cfg(target_os = "macos")]
    return macos::read(account);
    #[cfg(not(target_os = "macos"))]
    read_entry(account)
}

pub(crate) fn save(account: &str, value: &str) -> Result<(), CredentialError> {
    #[cfg(target_os = "macos")]
    return macos::save(account, Some(value));
    #[cfg(not(target_os = "macos"))]
    {
        keyring::Entry::new(SERVICE, account)?.set_password(value)?;
        Ok(())
    }
}

pub(crate) fn delete(account: &str) -> Result<(), CredentialError> {
    #[cfg(target_os = "macos")]
    return macos::save(account, None);
    #[cfg(not(target_os = "macos"))]
    match keyring::Entry::new(SERVICE, account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn read_entry(account: &str) -> Result<Stored<String>, CredentialError> {
    match keyring::Entry::new(SERVICE, account)?.get_password() {
        Ok(value) => Ok(Stored::Ready(value)),
        Err(keyring::Error::NoEntry) => Ok(Stored::Missing),
        Err(error) => Err(error.into()),
    }
}
