use crate::Error;

pub fn current_date() -> Result<String, Error> {
    Ok(time::OffsetDateTime::now_local()?.date().to_string())
}

pub fn validate_date(date: &str) -> Result<(), Error> {
    parse_date(date).map(|_| ())
}

// Mirrors ledger's parse_date (crates/ledger/src/lib.rs); keep both policies in sync.
pub(crate) fn parse_date(date: &str) -> Result<time::Date, Error> {
    let parsed = time::Date::parse(
        date,
        &time::macros::format_description!("[year]-[month]-[day]"),
    )
    .map_err(|_| Error::InvalidDate(date.to_owned()))?;
    if !(1..=9999).contains(&parsed.year()) || parsed.to_string() != date {
        return Err(Error::InvalidDate(date.to_owned()));
    }
    Ok(parsed)
}
