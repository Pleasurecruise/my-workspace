use crate::{Error, SERVICE, Stored};

#[cfg(target_os = "macos")]
use super::macos;

pub fn read(account: &str) -> Result<Stored<String>, Error> {
    #[cfg(target_os = "macos")]
    return macos::read(account);
    #[cfg(not(target_os = "macos"))]
    read_entry(account)
}

pub fn save(account: &str, value: &str) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    return macos::save(account, Some(value));
    #[cfg(not(target_os = "macos"))]
    {
        keyring::Entry::new(SERVICE, account)?.set_password(value)?;
        Ok(())
    }
}

pub fn delete(account: &str) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    return macos::save(account, None);
    #[cfg(not(target_os = "macos"))]
    match keyring::Entry::new(SERVICE, account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn read_entry(account: &str) -> Result<Stored<String>, Error> {
    match keyring::Entry::new(SERVICE, account)?.get_password() {
        Ok(value) => Ok(Stored::Ready(value)),
        Err(keyring::Error::NoEntry) => Ok(Stored::Missing),
        Err(error) => Err(error.into()),
    }
}
