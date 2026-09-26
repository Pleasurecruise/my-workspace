use std::collections::{BTreeMap, BTreeSet};

use chrono::TimeZone;
use rrule::{RRule, RRuleSet, Tz};
use time::{Date, Month};

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
    start: IcsDate,
    end: Option<IcsDate>,
    location: Option<String>,
    description: Option<String>,
    recurrence: Option<Recurrence>,
    excluded_dates: BTreeSet<Date>,
    cancelled: bool,
}

#[derive(Debug)]
struct Recurrence {
    rule: RRuleSet,
    until: Option<chrono::DateTime<chrono::Utc>>,
    source_zone: Tz,
}

/// An iCalendar DATE or DATE-TIME value, carried by DTSTART, DTEND, EXDATE, and UNTIL.
#[derive(Clone, Debug)]
struct IcsDate {
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

pub(crate) fn occurrences(input: &str, date: Date) -> Result<Vec<Occurrence>, String> {
    let events = parse(input)?;
    let mut seen = BTreeSet::new();
    let mut occurrences = Vec::new();
    for event in events {
        if event.cancelled {
            continue;
        }
        for source_date in event.candidate_dates(date) {
            if !event.occurs_on(source_date)? {
                continue;
            }
            let start = project_ics_date(&event.start, source_date, jiff::tz::TimeZone::system())?;
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
                    project_ics_date(&end, end_date, jiff::tz::TimeZone::system())
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

    fn occurs_on(&self, date: Date) -> Result<bool, String> {
        if date < self.start.date || self.excluded_dates.contains(&date) {
            return Ok(false);
        }
        let Some(recurrence) = &self.recurrence else {
            return Ok(date == self.start.date);
        };
        if let Some(until) = recurrence.until {
            let (hour, minute) = self.start.time.unwrap_or((0, 0));
            let instant =
                recurrence_bound(date, (hour as i8, minute as i8, 0), recurrence.source_zone)?;
            if instant > until {
                return Ok(false);
            }
        }
        let start = recurrence_datetime(date, (0, 0, 0), Tz::UTC)?;
        let end = recurrence_datetime(date, (23, 59, 59), Tz::UTC)?;
        let result = recurrence.rule.clone().after(start).before(end).all(2);
        if result.limited {
            return Err("iCalendar recurrence exceeded the evaluation limit".to_owned());
        }
        Ok(!result.dates.is_empty())
    }

    fn shifted_end(&self, occurrence_date: Date) -> Result<Option<IcsDate>, String> {
        let Some(end) = self.end.as_ref() else {
            return Ok(None);
        };
        let end_date = occurrence_date
            .checked_add(end.date - self.start.date)
            .ok_or_else(|| "iCalendar event end date is out of range".to_owned())?;
        Ok(Some(IcsDate {
            date: end_date,
            time: end.time,
            time_reference: end.time_reference.clone(),
        }))
    }
}

fn parse(input: &str) -> Result<Vec<Event>, String> {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let unfolded = icalendar::parser::unfold(&normalized);
    // icalendar selects TEXT decoders by case-sensitive property name and accepts
    // colonless extension properties. Preserve the importer's stricter contract.
    let mut content = String::with_capacity(unfolded.len());
    for line in unfolded.lines().filter(|line| !line.is_empty()) {
        if !line.contains(':') {
            return Err("iCalendar content line is missing ':'".to_owned());
        }
        let end = line.find([';', ':']).expect("content line has a colon");
        content.push_str(&line[..end].to_ascii_uppercase());
        content.push_str(&line[end..]);
        content.push('\n');
    }
    let mut roots = icalendar::parser::read_calendar_simple(&content)
        .map_err(|_| "invalid iCalendar document".to_owned())?;
    if roots.len() != 1 || roots[0].name.as_str() != "VCALENDAR" {
        return Err("expected one VCALENDAR section".to_owned());
    }
    let calendar = roots.remove(0);
    let mut events = Vec::new();
    for component in calendar.components {
        if component.name.as_str() == "VCALENDAR" {
            return Err("nested VCALENDAR is not supported".to_owned());
        }
        if component.name.as_str() != "VEVENT" {
            continue;
        }
        if component
            .components
            .iter()
            .any(|child| child.name.as_str() != "VALARM" || !child.components.is_empty())
        {
            return Err("unsupported VEVENT child component".to_owned());
        }
        let mut properties = BTreeMap::new();
        for property in component.properties {
            let (name, value) = event_property(property)?;
            properties.entry(name).or_insert_with(Vec::new).push(value);
        }
        events.push(parse_event(properties, events.len() + 1)?);
    }
    Ok(events)
}

fn event_property(
    property: icalendar::parser::Property<'_>,
) -> Result<(String, PropertyValue), String> {
    let name = property.name.to_string();
    let mut time_zone = None;
    if matches!(name.as_str(), "DTSTART" | "DTEND" | "EXDATE") {
        for parameter in property.params {
            let key = parameter.key.as_str().to_ascii_uppercase();
            let value = parameter
                .val
                .ok_or_else(|| format!("invalid {name} {key} parameter"))?;
            match key.as_str() {
                "TZID" if time_zone.is_none() => time_zone = Some(value.to_string()),
                "VALUE"
                    if value.as_str().eq_ignore_ascii_case("DATE")
                        || value.as_str().eq_ignore_ascii_case("DATE-TIME") => {}
                _ => return Err(format!("unsupported or duplicate {name} parameter {key}")),
            }
        }
    }
    Ok((
        name,
        PropertyValue {
            value: property.val.to_string(),
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
    let summary = take_one(&mut properties, "SUMMARY", number)?
        .value
        .replace('\n', " ");
    let start = parse_ics_date(take_one(&mut properties, "DTSTART", number)?)?;
    let end = take_optional(&mut properties, "DTEND", number)?
        .map(parse_ics_date)
        .transpose()?;
    let location = take_optional(&mut properties, "LOCATION", number)?
        .map(|value| value.value.replace('\n', " "));
    let description =
        take_optional(&mut properties, "DESCRIPTION", number)?.map(|value| value.value);
    let recurrence = take_optional(&mut properties, "RRULE", number)?
        .map(|value| parse_recurrence(&value.value, &start))
        .transpose()?;
    let mut excluded_dates = BTreeSet::new();
    for value in properties.remove("EXDATE").unwrap_or_default() {
        for excluded in value.value.split(',') {
            excluded_dates.insert(
                parse_ics_date(PropertyValue {
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

fn parse_ics_date(property: PropertyValue) -> Result<IcsDate, String> {
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
    let invalid_date = || format!("invalid iCalendar date {value}");
    if !raw.is_ascii() {
        return Err(invalid_date());
    }
    let date = raw.get(..8).ok_or_else(invalid_date)?;
    if !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_date());
    }
    let year = date[0..4].parse::<i32>().map_err(|_| invalid_date())?;
    let month = date[4..6]
        .parse::<u8>()
        .ok()
        .and_then(|month| Month::try_from(month).ok())
        .ok_or_else(invalid_date)?;
    let day = date[6..8].parse::<u8>().map_err(|_| invalid_date())?;
    let date = Date::from_calendar_date(year, month, day).map_err(|_| invalid_date())?;
    let time = if raw.len() == 8 {
        None
    } else {
        let clock = match raw.as_bytes().get(8) {
            Some(b'T') => &raw[9..],
            _ => return Err(format!("invalid iCalendar date-time {value}")),
        };
        Some(parse_ics_clock(clock, &value)?)
    };
    Ok(IcsDate {
        date,
        time,
        time_reference,
    })
}

/// Parses the `HHMM` or `HHMMSS` clock part of an iCalendar DATE-TIME value.
fn parse_ics_clock(clock: &str, value: &str) -> Result<(u8, u8), String> {
    let invalid = || format!("invalid iCalendar date-time {value}");
    if (clock.len() != 4 && clock.len() != 6) || !clock.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let hour = clock[0..2].parse::<u8>().map_err(|_| invalid())?;
    let minute = clock[2..4].parse::<u8>().map_err(|_| invalid())?;
    let second = if clock.len() == 6 {
        clock[4..6].parse::<u8>().map_err(|_| invalid())?
    } else {
        0
    };
    if hour > 23 || minute > 59 || second > 59 {
        return Err(invalid());
    }
    Ok((hour, minute))
}

fn project_ics_date(
    value: &IcsDate,
    occurrence_date: Date,
    target_time_zone: jiff::tz::TimeZone,
) -> Result<IcsDate, String> {
    let Some((hour, minute)) = value.time else {
        return Ok(IcsDate {
            date: occurrence_date,
            time: None,
            time_reference: TimeReference::Floating,
        });
    };
    if matches!(value.time_reference, TimeReference::Floating) {
        return Ok(IcsDate {
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
    let source = match &value.time_reference {
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
    Ok(IcsDate {
        date,
        time: Some((projected.hour() as u8, projected.minute() as u8)),
        time_reference: TimeReference::Floating,
    })
}

fn recurrence_datetime(
    date: Date,
    (hour, minute, second): (u32, u32, u32),
    zone: Tz,
) -> Result<chrono::DateTime<Tz>, String> {
    zone.with_ymd_and_hms(
        date.year(),
        date.month() as u32,
        date.day().into(),
        hour,
        minute,
        second,
    )
    .earliest()
    .ok_or_else(|| "iCalendar recurrence date-time cannot be resolved".to_owned())
}

fn recurrence_bound(
    date: Date,
    (hour, minute, second): (i8, i8, i8),
    zone: Tz,
) -> Result<chrono::DateTime<Tz>, String> {
    let year = i16::try_from(date.year())
        .map_err(|_| "iCalendar recurrence year is out of range".to_owned())?;
    // A source day can begin in a DST gap. Resolve it with the same compatible
    // policy as display projection instead of rejecting the entire day.
    let local = jiff::civil::DateTime::new(
        year,
        date.month() as i8,
        date.day() as i8,
        hour,
        minute,
        second,
        0,
    )
    .map_err(|error| format!("invalid recurrence bound: {error}"))?
    .in_tz(zone.name())
    .map_err(|error| format!("could not resolve recurrence bound: {error}"))?;
    let timestamp = local.timestamp();
    chrono::DateTime::from_timestamp(timestamp.as_second(), timestamp.subsec_nanosecond() as u32)
        .map(|instant| instant.with_timezone(&zone))
        .ok_or_else(|| "iCalendar recurrence bound is out of range".to_owned())
}

fn parse_recurrence(value: &str, start: &IcsDate) -> Result<Recurrence, String> {
    let zone = match &start.time_reference {
        TimeReference::Floating | TimeReference::Utc => Tz::UTC,
        TimeReference::Named(name) => name
            .parse::<chrono_tz::Tz>()
            .map(Tz::from)
            .map_err(|_| format!("unsupported recurrence time zone {name}"))?,
    };
    let (hour, minute) = start.time.unwrap_or((0, 0));
    // Enumerate source civil days in a UTC surrogate. Actual timezone projection
    // remains Jiff-compatible, including explicit DTSTARTs in a DST gap.
    let dt_start = recurrence_datetime(start.date, (hour.into(), minute.into(), 0), Tz::UTC)?;
    let mut fields = BTreeMap::new();
    for field in value.split(';') {
        let (name, value) = field
            .split_once('=')
            .ok_or_else(|| format!("invalid RRULE field {field}"))?;
        let name = name.to_ascii_uppercase();
        // Planner exposes at most one occurrence of an event per source day.
        if !matches!(
            name.as_str(),
            "FREQ" | "INTERVAL" | "BYDAY" | "BYMONTHDAY" | "UNTIL" | "COUNT"
        ) {
            return Err(format!("unsupported RRULE field {name}"));
        }
        if fields.insert(name.clone(), value.to_owned()).is_some() {
            return Err(format!("duplicate RRULE field {name}"));
        }
    }
    if !matches!(
        fields.get("FREQ").map(String::as_str),
        Some("DAILY" | "WEEKLY" | "MONTHLY" | "YEARLY")
    ) {
        return Err("unsupported or missing RRULE frequency".to_owned());
    }
    let mut absolute_until = None;
    if let Some(until) = fields.get_mut("UNTIL") {
        let parsed = parse_ics_date(PropertyValue {
            value: until.clone(),
            time_zone: None,
        })?;
        if parsed.time.is_some() && until.trim_end_matches('Z').len() == 13 {
            until.insert_str(13, "00");
        }
        if parsed.time.is_some() && matches!(parsed.time_reference, TimeReference::Utc) {
            let instant = chrono::NaiveDateTime::parse_from_str(until, "%Y%m%dT%H%M%SZ")
                .map_err(|_| "invalid RRULE UNTIL")?
                .and_utc();
            // Bound enumeration by source day, then compare the actual instant in
            // occurs_on. A local wall-clock cutoff would be ambiguous in a fold.
            *until = instant
                .with_timezone(&zone)
                .format("%Y%m%dT235959Z")
                .to_string();
            absolute_until = Some(instant);
        } else if parsed.time.is_none() {
            *until = recurrence_datetime(parsed.date, (23, 59, 59), Tz::UTC)?
                .format("%Y%m%dT%H%M%SZ")
                .to_string();
        } else {
            until.push('Z');
        }
    }
    let rule = fields
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(";");
    let rule = rule
        .parse::<RRule<rrule::Unvalidated>>()
        .map_err(|error| format!("invalid RRULE: {error}"))?;
    if rule.get_interval() == 0 || rule.get_count() == Some(0) {
        return Err("RRULE INTERVAL and COUNT must be positive integers".to_owned());
    }
    let rule = rule
        .validate(dt_start)
        .map_err(|error| format!("invalid RRULE: {error}"))?;
    Ok(Recurrence {
        rule: RRuleSet::new(dt_start).rrule(rule),
        until: absolute_until,
        source_zone: zone,
    })
}

fn format_time((hour, minute): (u8, u8)) -> String {
    format!("{hour:02}:{minute:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floating_until_uses_the_same_clock_as_start() {
        let input = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:floating\nSUMMARY:Floating\nDTSTART:20260901T090000\nRRULE:FREQ=DAILY;UNTIL=20260902T100000\nEND:VEVENT\nEND:VCALENDAR";
        let events = parse(input).unwrap();
        assert!(
            events[0]
                .occurs_on(time::macros::date!(2026 - 09 - 02))
                .unwrap()
        );
        assert!(
            !events[0]
                .occurs_on(time::macros::date!(2026 - 09 - 03))
                .unwrap()
        );
    }

    #[test]
    fn until_accepts_minute_precision() {
        for until in ["20260902T1000", "20260902T1000Z"] {
            let input = format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:short\nSUMMARY:Short\nDTSTART:20260901T0900\nRRULE:FREQ=DAILY;UNTIL={until}\nEND:VEVENT\nEND:VCALENDAR"
            );
            let events = parse(&input).unwrap();
            assert!(
                events[0]
                    .occurs_on(time::macros::date!(2026 - 09 - 02))
                    .unwrap()
            );
            assert!(
                !events[0]
                    .occurs_on(time::macros::date!(2026 - 09 - 03))
                    .unwrap()
            );
        }
    }

    #[test]
    fn gap_starts_preserve_civil_dates_and_count() {
        for (zone, start, first) in [
            (
                "Europe/London",
                "20260329T013000",
                time::macros::date!(2026 - 03 - 29),
            ),
            (
                "Australia/Lord_Howe",
                "20261004T021500",
                time::macros::date!(2026 - 10 - 04),
            ),
            (
                "America/Santiago",
                "20260906T003000",
                time::macros::date!(2026 - 09 - 06),
            ),
        ] {
            let input = format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:gap\nSUMMARY:Gap\nDTSTART;TZID={zone}:{start}\nRRULE:FREQ=DAILY;COUNT=2\nEND:VEVENT\nEND:VCALENDAR"
            );
            let events = parse(&input).unwrap();
            assert!(events[0].occurs_on(first).unwrap(), "{zone}");
            assert!(
                events[0].occurs_on(first.next_day().unwrap()).unwrap(),
                "{zone}"
            );
            assert!(
                !events[0]
                    .occurs_on(first + time::Duration::days(2))
                    .unwrap(),
                "{zone}"
            );
        }
    }

    #[test]
    fn absolute_until_compares_instants_across_a_fold() {
        for (until, expected) in [("20261025T001500Z", false), ("20261025T004500Z", true)] {
            let input = format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:fold\nSUMMARY:Fold\nDTSTART;TZID=Europe/London:20261024T013000\nRRULE:FREQ=DAILY;UNTIL={until}\nEND:VEVENT\nEND:VCALENDAR"
            );
            let events = parse(&input).unwrap();
            assert_eq!(
                events[0]
                    .occurs_on(time::macros::date!(2026 - 10 - 25))
                    .unwrap(),
                expected
            );
            assert!(
                !events[0]
                    .occurs_on(time::macros::date!(2026 - 10 - 26))
                    .unwrap()
            );
        }
    }

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
    fn rejects_malformed_ics() {
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
    fn projects_time_zones() {
        let calendar = "BEGIN:VCALENDAR\nBEGIN:VTIMEZONE\nTZID:Europe/London\nBEGIN:DAYLIGHT\nDTSTART:19700329T010000\nRRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU\nEND:DAYLIGHT\nEND:VTIMEZONE\nBEGIN:VEVENT\nUID:training\nSUMMARY:Training\nDTSTART;TZID=Europe/London:20260907T073000\nDTEND;TZID=Europe/London:20260907T083000\nRRULE:FREQ=WEEKLY;COUNT=12\nEND:VEVENT\nEND:VCALENDAR\n";
        assert!(validate(calendar).is_ok());

        let london = parse_ics_date(PropertyValue {
            value: "20260907T073000".to_owned(),
            time_zone: Some("Europe/London".to_owned()),
        })
        .unwrap();
        let shanghai = jiff::tz::TimeZone::get("Asia/Shanghai").unwrap();
        let projected = project_ics_date(
            &london,
            time::macros::date!(2026 - 09 - 07),
            shanghai.clone(),
        )
        .unwrap();
        assert_eq!(projected.date, time::macros::date!(2026 - 09 - 07));
        assert_eq!(projected.time, Some((14, 30)));

        let utc = parse_ics_date(PropertyValue {
            value: "20260907T233000Z".to_owned(),
            time_zone: None,
        })
        .unwrap();
        let projected =
            project_ics_date(&utc, time::macros::date!(2026 - 09 - 07), shanghai).unwrap();
        assert_eq!(projected.date, time::macros::date!(2026 - 09 - 08));
        assert_eq!(projected.time, Some((7, 30)));
    }

    #[test]
    fn exclusions_do_not_extend_count() {
        let input = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:daily\nSUMMARY:Daily\nDTSTART;VALUE=DATE:20260901\nRRULE:FREQ=DAILY;COUNT=2\nEXDATE;VALUE=DATE:20260902\nEND:VEVENT\nEND:VCALENDAR\n";
        assert_eq!(
            occurrences(input, time::macros::date!(2026 - 09 - 01))
                .unwrap()
                .len(),
            1
        );
        assert!(
            occurrences(input, time::macros::date!(2026 - 09 - 02))
                .unwrap()
                .is_empty()
        );
        assert!(
            occurrences(input, time::macros::date!(2026 - 09 - 03))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn parses_quoted_parameters_and_ignores_alarm_properties() {
        let input = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:daily\r\nSUMMARY;ALTREP=\"https://example.com/a;b\":Study\r\nDTSTART;TZID=\"Europe/London\":20260901T090000\r\nBEGIN:VALARM\r\nSUMMARY:Reminder\r\nDESCRIPTION:Not the event description\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = parse(input).unwrap();
        assert_eq!(events[0].summary, "Study");
        assert_eq!(events[0].description, None);
    }

    #[test]
    fn daily_rules_apply_weekday_filters() {
        let input = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:daily\nSUMMARY:Weekdays\nDTSTART:20260904\nRRULE:FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;COUNT=2\nEND:VEVENT\nEND:VCALENDAR\n";
        assert!(
            occurrences(input, time::macros::date!(2026 - 09 - 05))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            occurrences(input, time::macros::date!(2026 - 09 - 07))
                .unwrap()
                .len(),
            1
        );
        assert!(
            occurrences(input, time::macros::date!(2026 - 09 - 08))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_mismatched_components_and_duplicate_rules() {
        for body in [
            "END:VTODO\nEND:VCALENDAR",
            "END:VEVENT\nEND:VTODO",
            "RRULE:FREQ=DAILY;COUNT=2;COUNT=3\nEND:VEVENT\nEND:VCALENDAR",
        ] {
            let input = format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:event\nSUMMARY:Event\nDTSTART:20260901\n{body}\n"
            );
            assert!(validate(&input).is_err(), "{body}");
        }
        assert!(
            validate("BEGIN:VCALENDAR\nEND:VCALENDAR\nBEGIN:VCALENDAR\nEND:VCALENDAR\n").is_err()
        );
    }

    #[test]
    fn respects_month_lengths_until_and_dst() {
        let monthly = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:monthly\nSUMMARY:Monthly\nDTSTART:20260131\nRRULE:FREQ=MONTHLY;COUNT=2\nEND:VEVENT\nEND:VCALENDAR\n";
        assert!(
            occurrences(monthly, time::macros::date!(2026 - 02 - 28))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            occurrences(monthly, time::macros::date!(2026 - 03 - 31))
                .unwrap()
                .len(),
            1
        );
        let weekly = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:weekly\nSUMMARY:Weekly\nDTSTART;TZID=Europe/London:20261018T090000\nRRULE:FREQ=WEEKLY;UNTIL=20261025\nEND:VEVENT\nEND:VCALENDAR\n";
        let event = parse(weekly).unwrap().remove(0);
        assert!(
            event
                .occurs_on(time::macros::date!(2026 - 10 - 25))
                .unwrap()
        );
        assert!(
            !event
                .occurs_on(time::macros::date!(2026 - 11 - 01))
                .unwrap()
        );
        let winter = project_ics_date(
            &event.start,
            time::macros::date!(2026 - 10 - 25),
            jiff::tz::TimeZone::UTC,
        )
        .unwrap();
        assert_eq!(winter.time, Some((9, 0)));
    }

    #[test]
    fn midnight_dst_does_not_hide_daytime_events() {
        let input = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:daily\nSUMMARY:Daily\nDTSTART;TZID=America/Santiago:20260901T090000\nRRULE:FREQ=DAILY\nEND:VEVENT\nEND:VCALENDAR\n";
        let event = parse(input).unwrap().remove(0);
        for date in [
            time::macros::date!(2026 - 09 - 05),
            time::macros::date!(2026 - 09 - 06),
            time::macros::date!(2026 - 09 - 07),
        ] {
            assert!(event.occurs_on(date).unwrap());
            assert_eq!(occurrences(input, date).unwrap().len(), 1);
        }
    }

    #[test]
    fn decodes_case_insensitive_text_once() {
        for name in ["SUMMARY", "summary", "Summary"] {
            let input = format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:text\n{name}:Study\\, review\\nmore\\\\notes\nDTSTART:20260901\nEND:VEVENT\nEND:VCALENDAR\n"
            );
            let event = parse(&input).unwrap().remove(0);
            assert_eq!(event.summary, "Study, review more\\notes");
        }
    }

    #[test]
    fn rejects_lenient_library_inputs() {
        for body in [
            "NONSENSE",
            "RRULE:FREQ=DAILY;COUNT=0",
            "RRULE:FREQ=DAILY;INTERVAL=0",
            "BEGIN:VALARM\nBEGIN:VEVENT\nUID:nested\nSUMMARY:Nested\nDTSTART:20260901\nEND:VEVENT\nEND:VALARM",
        ] {
            let input = format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:invalid\nSUMMARY:Invalid\nDTSTART:20260901\n{body}\nEND:VEVENT\nEND:VCALENDAR\n"
            );
            assert!(validate(&input).is_err(), "{body}");
        }
    }
}
