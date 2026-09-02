use crate::Error;

pub fn current_date() -> Result<String, Error> {
    Ok(time::OffsetDateTime::now_local()?.date().to_string())
}

pub fn validate_date(date: &str) -> Result<(), Error> {
    parse_date(date).map(|_| ())
}

pub(crate) fn parse_date(date: &str) -> Result<time::Date, Error> {
    time::Date::parse(
        date,
        &time::macros::format_description!("[year]-[month]-[day]"),
    )
    .map_err(|_| Error::InvalidDate(date.to_owned()))
}

pub fn next_rollover_delay() -> Result<std::time::Duration, Error> {
    let now = time::OffsetDateTime::now_local()?;
    let tomorrow = now.date().next_day().ok_or(Error::DateOverflow)?;
    let midnight = tomorrow.midnight();
    let approximate = midnight.assume_offset(now.offset());
    let midnight_offset = time::UtcOffset::local_offset_at(approximate)?;
    Ok((midnight.assume_offset(midnight_offset) - now).unsigned_abs())
}
