use crate::{CredentialError, Stored};
use diesel::prelude::*;

diesel::table! {
    credentials (account) {
        account -> Text,
        value -> Text,
    }
}

pub(crate) fn read(account: &str) -> Result<Stored<String>, CredentialError> {
    let mut connection = vesper_database::open(&vesper_database::shared_path()?)?;
    match credentials::table
        .find(account)
        .select(credentials::value)
        .first::<String>(&mut connection)
        .optional()?
    {
        Some(value) => Ok(Stored::Ready(value)),
        None => Ok(Stored::Missing),
    }
}

pub(crate) fn save(account: &str, value: &str) -> Result<(), CredentialError> {
    let mut connection = vesper_database::open(&vesper_database::shared_path()?)?;
    diesel::insert_into(credentials::table)
        .values((
            credentials::account.eq(account),
            credentials::value.eq(value),
        ))
        .on_conflict(credentials::account)
        .do_update()
        .set(credentials::value.eq(value))
        .execute(&mut connection)?;
    Ok(())
}

pub(crate) fn delete(account: &str) -> Result<(), CredentialError> {
    let mut connection = vesper_database::open(&vesper_database::shared_path()?)?;
    diesel::delete(credentials::table.find(account)).execute(&mut connection)?;
    Ok(())
}
