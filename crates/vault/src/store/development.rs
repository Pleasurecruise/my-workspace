use crate::{Error, Stored};
use diesel::prelude::*;

diesel::table! {
    credentials (account) {
        account -> Text,
        value -> Text,
    }
}

pub fn read(account: &str) -> Result<Stored<String>, Error> {
    let mut connection = database::open(&database::path()?)?;
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

pub fn save(account: &str, value: &str) -> Result<(), Error> {
    let mut connection = database::open(&database::path()?)?;
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

pub fn delete(account: &str) -> Result<(), Error> {
    let mut connection = database::open(&database::path()?)?;
    diesel::delete(credentials::table.find(account)).execute(&mut connection)?;
    Ok(())
}
