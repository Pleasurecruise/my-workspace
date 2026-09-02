use std::collections::{BTreeMap, BTreeSet};

use time::{Date, Month, Weekday};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Occurrence {
    pub(crate) key: String,
    pub(crate) text: String,
    pub(crate) details: Details,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Details {
    pub(crate) calendar: String,
    pub(crate) start_date: String,
    pub(crate) start_time: Option<String>,
    pub(crate) end_date: Option<String>,
    pub(crate) end_time: Option<String>,
    pub(crate) location: Option<String>,
    pub(crate) description: Option<String>,
}

#[derive(Debug)]
struct Event {
    uid: String,
    summary: String,
    start: Start,
    end: Option<Start>,
    location: Option<String>,
    description: Option<String>,
    recurrence: Option<Recurrence>,
    excluded_dates: BTreeSet<Date>,
    cancelled: bool,
}

#[derive(Clone, Debug)]
struct Start {
    date: Date,
    time: Option<(u8, u8)>,
    time_reference: TimeReference,
}

#[derive(Clone, Debug)]
enum TimeReference {
    Floating,
    Utc,
    Named(String),
}

#[derive(Debug)]
struct PropertyValue {
    value: String,
    time_zone: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug)]
struct Recurrence {
    frequency: Frequency,
    interval: i64,
    weekdays: Vec<Weekday>,
    month_days: BTreeSet<u8>,
    until: Option<Date>,
    count: Option<usize>,
}

pub(crate) fn occurrences(input: &str, date: Date) -> Result<Vec<Occurrence>, String> {
    let events = parse(input)?;
    let mut seen = BTreeSet::new();
    let mut occurrences = Vec::new();
    for event in events {
        if event.cancelled {
            continue;
        }
        for source_date in event.candidate_dates(date) {
            if !event.occurs_on(source_date) {
                continue;
            }
            let start = project_start(&event.start, source_date, jiff::tz::TimeZone::system())?;
            if start.date != date {
                continue;
            }
            let key = format!("{}:{}", event.uid, source_date);
            if !seen.insert(key.clone()) {
                continue;
            }
            let end = event
                .shifted_end(source_date)?
                .map(|end| {
                    let end_date = end.date;
                    project_start(&end, end_date, jiff::tz::TimeZone::system())
                })
                .transpose()?;
            let text = match start.time {
                Some((hour, minute)) => format!("{hour:02}:{minute:02} {}", event.summary),
                None => event.summary.clone(),
            };
            occurrences.push(Occurrence {
                key,
                text,
                details: Details {
                    calendar: String::new(),
                    start_date: start.date.to_string(),
                    start_time: start.time.map(format_time),
                    end_date: end.as_ref().map(|end| end.date.to_string()),
                    end_time: end.and_then(|end| end.time.map(format_time)),
                    location: event.location.clone(),
                    description: event.description.clone(),
                },
            });
        }
    }
    Ok(occurrences)
}

pub(crate) fn validate(input: &str) -> Result<(), String> {
    parse(input).map(|_| ())
}

impl Event {
    fn candidate_dates(&self, target: Date) -> Vec<Date> {
        if self.start.time.is_none() || matches!(self.start.time_reference, TimeReference::Floating)
        {
            return vec![target];
        }
        (-2..=2)
            .filter_map(|offset| target.checked_add(time::Duration::days(offset)))
            .collect()
    }

    fn occurs_on(&self, date: Date) -> bool {
        if date < self.start.date || self.excluded_dates.contains(&date) {
            return false;
        }
        let Some(recurrence) = &self.recurrence else {
            return date == self.start.date;
        };
        if recurrence.until.is_some_and(|until| date > until)
            || !recurrence.matches(self.start.date, date)
        {
            return false;
        }
        let Some(count) = recurrence.count else {
            return true;
        };
        let mut occurrence_count = 0;
        let mut cursor = self.start.date;
        while cursor <= date {
            if recurrence.matches(self.start.date, cursor) && !self.excluded_dates.contains(&cursor)
            {
                occurrence_count += 1;
                if cursor == date {
                    return occurrence_count <= count;
                }
            }
            let Some(next) = cursor.next_day() else {
                return false;
            };
            cursor = next;
        }
        false
    }

    fn shifted_end(&self, occurrence_date: Date) -> Result<Option<Start>, String> {
        let Some(end) = self.end.as_ref() else {
            return Ok(None);
        };
        let end_date = occurrence_date
            .checked_add(end.date - self.start.date)
            .ok_or_else(|| "iCalendar event end date is out of range".to_owned())?;
        Ok(Some(Start {
            date: end_date,
            time: end.time,
            time_reference: end.time_reference.clone(),
        }))
    }
}

impl Recurrence {
    fn matches(&self, start: Date, date: Date) -> bool {
        match self.frequency {
            Frequency::Daily => (date - start).whole_days() % self.interval == 0,
            Frequency::Weekly => {
                let start_week = start - time::Duration::days(weekday_index(start.weekday()));
                let date_week = date - time::Duration::days(weekday_index(date.weekday()));
                let weekday_matches = if self.weekdays.is_empty() {
                    date.weekday() == start.weekday()
                } else {
                    self.weekdays.contains(&date.weekday())
                };
                weekday_matches && (date_week - start_week).whole_weeks() % self.interval == 0
            }
            Frequency::Monthly => {
                let months = (date.year() - start.year()) as i64 * 12 + date.month() as i64
                    - start.month() as i64;
                let day_matches = if self.month_days.is_empty() {
                    date.day() == start.day()
                } else {
                    self.month_days.contains(&date.day())
                };
                months % self.interval == 0 && day_matches
            }
            Frequency::Yearly => {
                let years = (date.year() - start.year()) as i64;
                years % self.interval == 0
                    && date.month() == start.month()
                    && date.day() == start.day()
            }
        }
    }
}

fn parse(input: &str) -> Result<Vec<Event>, String> {
    let lines = unfold(input);
    let mut events = Vec::new();
    let mut properties: Option<BTreeMap<String, Vec<PropertyValue>>> = None;
    let mut calendar_open = false;
    let mut calendar_closed = false;
    for line in lines {
        match line.as_str() {
            "BEGIN:VCALENDAR" => {
                if calendar_open || calendar_closed {
                    return Err("multiple VCALENDAR sections are not supported".to_owned());
                }
                calendar_open = true;
            }
            "END:VCALENDAR" => {
                if !calendar_open || properties.is_some() {
                    return Err("unexpected END:VCALENDAR".to_owned());
                }
                calendar_open = false;
                calendar_closed = true;
            }
            "BEGIN:VEVENT" => {
                if !calendar_open {
                    return Err("VEVENT must be inside VCALENDAR".to_owned());
                }
                if properties.is_some() {
                    return Err("nested VEVENT is not supported".to_owned());
                }
                properties = Some(BTreeMap::new());
            }
            "END:VEVENT" => {
                let values = properties
                    .take()
                    .ok_or_else(|| "END:VEVENT without BEGIN:VEVENT".to_owned())?;
                events.push(parse_event(values, events.len() + 1)?);
            }
            _ => {
                if let Some(values) = properties.as_mut() {
                    let (name, value) = property(&line)?;
                    values.entry(name).or_default().push(value);
                }
            }
        }
    }
    if properties.is_some() {
        return Err("VEVENT is missing END:VEVENT".to_owned());
    }
    if !calendar_closed {
        return Err("VCALENDAR is missing END:VCALENDAR".to_owned());
    }
    Ok(events)
}

fn unfold(input: &str) -> Vec<String> {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = Vec::new();
    for line in normalized.lines() {
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            lines.last_mut().expect("line exists").push_str(&line[1..]);
        } else {
            lines.push(line.to_owned());
        }
    }
    lines
}

fn property(line: &str) -> Result<(String, PropertyValue), String> {
    let (head, value) = line
        .split_once(':')
        .ok_or_else(|| format!("invalid content line {line}"))?;
    let mut parts = head.split(';');
    let name = parts
        .next()
        .expect("split always returns one value")
        .to_ascii_uppercase();
    let mut time_zone = None;
    if matches!(name.as_str(), "DTSTART" | "DTEND" | "EXDATE") {
        for parameter in parts {
            let (parameter_name, parameter_value) = parameter
                .split_once('=')
                .ok_or_else(|| format!("invalid {name} parameter {parameter}"))?;
            if parameter_name.eq_ignore_ascii_case("TZID") {
                if time_zone.replace(parameter_value.to_owned()).is_some() {
                    return Err(format!("duplicate {name} TZID parameter"));
                }
                continue;
            }
            if !parameter_name.eq_ignore_ascii_case("VALUE")
                || (!parameter_value.eq_ignore_ascii_case("DATE")
                    && !parameter_value.eq_ignore_ascii_case("DATE-TIME"))
            {
                return Err(format!("unsupported {name} parameter {parameter}"));
            }
        }
    }
    Ok((
        name,
        PropertyValue {
            value: value.to_owned(),
            time_zone,
        },
    ))
}

fn parse_event(
    mut properties: BTreeMap<String, Vec<PropertyValue>>,
    number: usize,
) -> Result<Event, String> {
    for unsupported in ["RECURRENCE-ID", "RDATE", "DURATION"] {
        if properties.contains_key(unsupported) {
            return Err(format!(
                "VEVENT {number} uses unsupported {unsupported} semantics"
            ));
        }
    }
    let uid = take_one(&mut properties, "UID", number)?.value;
    let summary = unescape(&take_one(&mut properties, "SUMMARY", number)?.value, ' ');
    let start = parse_start(take_one(&mut properties, "DTSTART", number)?)?;
    let end = take_optional(&mut properties, "DTEND", number)?
        .map(parse_start)
        .transpose()?;
    let location = take_optional(&mut properties, "LOCATION", number)?
        .map(|value| unescape(&value.value, ' '));
    let description = take_optional(&mut properties, "DESCRIPTION", number)?
        .map(|value| unescape(&value.value, '\n'));
    let recurrence = take_optional(&mut properties, "RRULE", number)?
        .map(|value| parse_recurrence(&value.value))
        .transpose()?;
    let mut excluded_dates = BTreeSet::new();
    for value in properties.remove("EXDATE").unwrap_or_default() {
        for excluded in value.value.split(',') {
            excluded_dates.insert(
                parse_start(PropertyValue {
                    value: excluded.to_owned(),
                    time_zone: value.time_zone.clone(),
                })?
                .date,
            );
        }
    }
    let cancelled = take_optional(&mut properties, "STATUS", number)?
        .is_some_and(|status| status.value.eq_ignore_ascii_case("CANCELLED"));
    Ok(Event {
        uid,
        summary,
        start,
        end,
        location,
        description,
        recurrence,
        excluded_dates,
        cancelled,
    })
}

fn take_one(
    properties: &mut BTreeMap<String, Vec<PropertyValue>>,
    name: &str,
    number: usize,
) -> Result<PropertyValue, String> {
    take_optional(properties, name, number)?
        .ok_or_else(|| format!("VEVENT {number} is missing {name}"))
}

fn take_optional(
    properties: &mut BTreeMap<String, Vec<PropertyValue>>,
    name: &str,
    number: usize,
) -> Result<Option<PropertyValue>, String> {
    let Some(mut values) = properties.remove(name) else {
        return Ok(None);
    };
    if values.len() != 1 {
        return Err(format!("VEVENT {number} has multiple {name} values"));
    }
    Ok(values.pop().filter(|value| !value.value.trim().is_empty()))
}

fn parse_start(property: PropertyValue) -> Result<Start, String> {
    let value = property.value;
    let (raw, time_reference) = if let Some(raw) = value.strip_suffix('Z') {
        if property.time_zone.is_some() {
            return Err(format!("iCalendar date-time {value} has both UTC and TZID"));
        }
        (raw, TimeReference::Utc)
    } else if let Some(time_zone) = property.time_zone {
        jiff::tz::TimeZone::get(&time_zone)
            .map_err(|_| format!("unknown iCalendar time zone {time_zone}"))?;
        (value.as_str(), TimeReference::Named(time_zone))
    } else {
        (value.as_str(), TimeReference::Floating)
    };
    if !raw.is_ascii() {
        return Err(format!("invalid iCalendar date {value}"));
    }
    let date = raw
        .get(..8)
        .ok_or_else(|| format!("invalid iCalendar date {value}"))?;
    if !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("invalid iCalendar date {value}"));
    }
    let year = date[0..4]
        .parse::<i32>()
        .map_err(|_| format!("invalid iCalendar date {value}"))?;
    let month = date[4..6]
        .parse::<u8>()
        .ok()
        .and_then(|month| Month::try_from(month).ok())
        .ok_or_else(|| format!("invalid iCalendar date {value}"))?;
    let day = date[6..8]
        .parse::<u8>()
        .map_err(|_| format!("invalid iCalendar date {value}"))?;
    let date = Date::from_calendar_date(year, month, day)
        .map_err(|_| format!("invalid iCalendar date {value}"))?;
    let time = if raw.len() == 8 {
        None
    } else {
        let clock = raw
            .strip_prefix(&format!("{}T", &raw[..8]))
            .ok_or_else(|| format!("invalid iCalendar date-time {value}"))?;
        if clock.len() != 4 && clock.len() != 6 {
            return Err(format!("invalid iCalendar date-time {value}"));
        }
        if !clock.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("invalid iCalendar date-time {value}"));
        }
        let hour = clock[0..2]
            .parse::<u8>()
            .map_err(|_| format!("invalid iCalendar date-time {value}"))?;
        let minute = clock[2..4]
            .parse::<u8>()
            .map_err(|_| format!("invalid iCalendar date-time {value}"))?;
        let second = if clock.len() == 6 {
            clock[4..6]
                .parse::<u8>()
                .map_err(|_| format!("invalid iCalendar date-time {value}"))?
        } else {
            0
        };
        if hour > 23 || minute > 59 || second > 59 {
            return Err(format!("invalid iCalendar date-time {value}"));
        }
        Some((hour, minute))
    };
    Ok(Start {
        date,
        time,
        time_reference,
    })
}

fn project_start(
    start: &Start,
    occurrence_date: Date,
    target_time_zone: jiff::tz::TimeZone,
) -> Result<Start, String> {
    let Some((hour, minute)) = start.time else {
        return Ok(Start {
            date: occurrence_date,
            time: None,
            time_reference: TimeReference::Floating,
        });
    };
    if matches!(start.time_reference, TimeReference::Floating) {
        return Ok(Start {
            date: occurrence_date,
            time: Some((hour, minute)),
            time_reference: TimeReference::Floating,
        });
    }
    let year = i16::try_from(occurrence_date.year())
        .map_err(|_| "iCalendar year is outside the supported time-zone range".to_owned())?;
    let source = jiff::civil::DateTime::new(
        year,
        occurrence_date.month() as i8,
        occurrence_date.day() as i8,
        hour as i8,
        minute as i8,
        0,
        0,
    )
    .map_err(|error| format!("invalid iCalendar date-time: {error}"))?;
    let source = match &start.time_reference {
        TimeReference::Floating => unreachable!("floating dates return before projection"),
        TimeReference::Utc => source.in_tz("UTC"),
        TimeReference::Named(time_zone) => source.in_tz(time_zone),
    }
    .map_err(|error| format!("could not resolve iCalendar time zone: {error}"))?;
    let projected = source.with_time_zone(target_time_zone);
    let month = Month::try_from(projected.month() as u8)
        .map_err(|_| "projected iCalendar month is invalid".to_owned())?;
    let date = Date::from_calendar_date(projected.year().into(), month, projected.day() as u8)
        .map_err(|_| "projected iCalendar date is invalid".to_owned())?;
    Ok(Start {
        date,
        time: Some((projected.hour() as u8, projected.minute() as u8)),
        time_reference: TimeReference::Floating,
    })
}

fn parse_recurrence(value: &str) -> Result<Recurrence, String> {
    let mut fields = BTreeMap::new();
    for field in value.split(';') {
        let (name, field_value) = field
            .split_once('=')
            .ok_or_else(|| format!("invalid RRULE field {field}"))?;
        let name = name.to_ascii_uppercase();
        if name.is_empty() || field_value.is_empty() {
            return Err(format!("invalid RRULE field {field}"));
        }
        if !matches!(
            name.as_str(),
            "FREQ" | "INTERVAL" | "BYDAY" | "BYMONTHDAY" | "UNTIL" | "COUNT"
        ) {
            return Err(format!("unsupported RRULE field {name}"));
        }
        if fields.insert(name.clone(), field_value).is_some() {
            return Err(format!("duplicate RRULE field {name}"));
        }
    }
    let frequency = match fields.get("FREQ").copied() {
        Some("DAILY") => Frequency::Daily,
        Some("WEEKLY") => Frequency::Weekly,
        Some("MONTHLY") => Frequency::Monthly,
        Some("YEARLY") => Frequency::Yearly,
        Some(other) => return Err(format!("unsupported RRULE frequency {other}")),
        None => return Err("RRULE is missing FREQ".to_owned()),
    };
    let interval = fields
        .get("INTERVAL")
        .map(|value| value.parse::<i64>())
        .transpose()
        .map_err(|_| "RRULE INTERVAL must be a positive integer".to_owned())?
        .unwrap_or(1);
    if interval < 1 {
        return Err("RRULE INTERVAL must be a positive integer".to_owned());
    }
    let weekdays = fields
        .get("BYDAY")
        .map(|value| {
            value
                .split(',')
                .map(parse_weekday)
                .collect::<Result<_, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let month_days = fields
        .get("BYMONTHDAY")
        .map(|value| {
            value
                .split(',')
                .map(|day| {
                    day.parse::<u8>()
                        .ok()
                        .filter(|day| (1..=31).contains(day))
                        .ok_or_else(|| format!("unsupported RRULE BYMONTHDAY {day}"))
                })
                .collect::<Result<_, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let until = fields
        .get("UNTIL")
        .map(|value| {
            parse_start(PropertyValue {
                value: (*value).to_owned(),
                time_zone: None,
            })
            .map(|start| start.date)
        })
        .transpose()?;
    let count = fields
        .get("COUNT")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| "RRULE COUNT must be a positive integer".to_owned())?;
    if count == Some(0) {
        return Err("RRULE COUNT must be a positive integer".to_owned());
    }
    Ok(Recurrence {
        frequency,
        interval,
        weekdays,
        month_days,
        until,
        count,
    })
}

fn parse_weekday(value: &str) -> Result<Weekday, String> {
    match value {
        "MO" => Ok(Weekday::Monday),
        "TU" => Ok(Weekday::Tuesday),
        "WE" => Ok(Weekday::Wednesday),
        "TH" => Ok(Weekday::Thursday),
        "FR" => Ok(Weekday::Friday),
        "SA" => Ok(Weekday::Saturday),
        "SU" => Ok(Weekday::Sunday),
        _ => Err(format!("unsupported RRULE BYDAY {value}")),
    }
}

fn weekday_index(weekday: Weekday) -> i64 {
    match weekday {
        Weekday::Monday => 0,
        Weekday::Tuesday => 1,
        Weekday::Wednesday => 2,
        Weekday::Thursday => 3,
        Weekday::Friday => 4,
        Weekday::Saturday => 5,
        Weekday::Sunday => 6,
    }
}

fn unescape(value: &str, newline: char) -> String {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('n' | 'N') => output.push(newline),
            Some(character) => output.push(character),
            None => output.push('\\'),
        }
    }
    output
}

fn format_time((hour, minute): (u8, u8)) -> String {
    format!("{hour:02}:{minute:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_weekly_rules() {
        let input = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:study\r\nSUMMARY:Study\\, review\r\nDTSTART:20260901T090000\r\nDTEND:20260901T103000\r\nLOCATION:Library\\; room 2\r\nDESCRIPTION:Read chapter 1\\nBring notes\r\nRRULE:FREQ=WEEKLY;BYDAY=TU,TH\r\nEXDATE:20260903T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        assert_eq!(
            occurrences(input, time::macros::date!(2026 - 09 - 01)).unwrap(),
            vec![Occurrence {
                key: "study:2026-09-01".to_owned(),
                text: "09:00 Study, review".to_owned(),
                details: Details {
                    calendar: String::new(),
                    start_date: "2026-09-01".to_owned(),
                    start_time: Some("09:00".to_owned()),
                    end_date: Some("2026-09-01".to_owned()),
                    end_time: Some("10:30".to_owned()),
                    location: Some("Library; room 2".to_owned()),
                    description: Some("Read chapter 1\nBring notes".to_owned()),
                },
            }]
        );
        assert!(
            occurrences(input, time::macros::date!(2026 - 09 - 03))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            occurrences(input, time::macros::date!(2026 - 09 - 08))
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn respects_daily_count() {
        let input = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:daily\nSUMMARY:Morning \n routine\nDTSTART:20260901T073000\nRRULE:FREQ=DAILY;COUNT=2\nEND:VEVENT\nEND:VCALENDAR\n";
        assert_eq!(
            occurrences(input, time::macros::date!(2026 - 09 - 02)).unwrap()[0].text,
            "07:30 Morning routine"
        );
        assert!(
            occurrences(input, time::macros::date!(2026 - 09 - 03))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_malformed_calendar_data_without_panicking() {
        for input in [
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20日60901\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901\nRRULE:FREQ=DAILY;COUNT\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901\nRRULE:FREQ=DAILY;BYHOUR=9\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901\nEND:VEVENT\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901T090061\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901\nDTSTART:20260902\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nRECURRENCE-ID:20260902\nSUMMARY:Moved\nDTSTART:20260903\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901\nRDATE:20260902\nEND:VEVENT\nEND:VCALENDAR\n",
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:bad\nSUMMARY:Bad\nDTSTART:20260901T090000\nDURATION:PT1H\nEND:VEVENT\nEND:VCALENDAR\n",
        ] {
            assert!(validate(input).is_err());
        }
    }

    #[test]
    fn projects_named_and_utc_times_into_the_target_zone() {
        let calendar = "BEGIN:VCALENDAR\nBEGIN:VTIMEZONE\nTZID:Europe/London\nBEGIN:DAYLIGHT\nDTSTART:19700329T010000\nRRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU\nEND:DAYLIGHT\nEND:VTIMEZONE\nBEGIN:VEVENT\nUID:training\nSUMMARY:Training\nDTSTART;TZID=Europe/London:20260907T073000\nDTEND;TZID=Europe/London:20260907T083000\nRRULE:FREQ=WEEKLY;COUNT=12\nEND:VEVENT\nEND:VCALENDAR\n";
        assert!(validate(calendar).is_ok());

        let london = parse_start(PropertyValue {
            value: "20260907T073000".to_owned(),
            time_zone: Some("Europe/London".to_owned()),
        })
        .unwrap();
        let shanghai = jiff::tz::TimeZone::get("Asia/Shanghai").unwrap();
        let projected = project_start(
            &london,
            time::macros::date!(2026 - 09 - 07),
            shanghai.clone(),
        )
        .unwrap();
        assert_eq!(projected.date, time::macros::date!(2026 - 09 - 07));
        assert_eq!(projected.time, Some((14, 30)));

        let utc = parse_start(PropertyValue {
            value: "20260907T233000Z".to_owned(),
            time_zone: None,
        })
        .unwrap();
        let projected = project_start(&utc, time::macros::date!(2026 - 09 - 07), shanghai).unwrap();
        assert_eq!(projected.date, time::macros::date!(2026 - 09 - 08));
        assert_eq!(projected.time, Some((7, 30)));
    }
}
