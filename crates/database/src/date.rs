/// Parses the `YYYY-MM-DD` text stored in date columns. The round trip rejects
/// unpadded or out-of-range values that `time` would otherwise normalize.
pub fn parse(value: &str) -> Option<time::Date> {
    let date = time::Date::parse(
        value,
        &time::macros::format_description!("[year]-[month]-[day]"),
    )
    .ok()?;
    if !(1..=9999).contains(&date.year()) || date.to_string() != value {
        return None;
    }
    Some(date)
}
