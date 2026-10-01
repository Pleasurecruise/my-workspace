use crate::Error;

pub fn current_date() -> Result<String, Error> {
    Ok(time::OffsetDateTime::now_local()?.date().to_string())
}

pub fn validate_date(date: &str) -> Result<(), Error> {
    parse_date(date).map(|_| ())
}

pub(crate) fn parse_date(date: &str) -> Result<time::Date, Error> {
    database::date::parse(date).ok_or_else(|| Error::InvalidDate(date.to_owned()))
}
