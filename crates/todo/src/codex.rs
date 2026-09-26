use crate::{Details, Error, Item};
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use serde::Deserialize;
use std::{collections::BTreeSet, time::Duration};

pub(crate) const ENDPOINT: &str = "https://codex-resets.com/api/v1/resets";

#[derive(Deserialize)]
struct Page {
    data: Vec<Reset>,
    pagination: Pagination,
}

#[derive(Deserialize)]
struct Pagination {
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct Reset {
    id: String,
    reset_type: ResetType,
    announced_at: String,
    text: String,
    source: Source,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResetType {
    Regular,
    Banked,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Source {
    XPost { url: String },
    Observed { url: Option<String> },
}

pub(crate) async fn read(date: &str, zone: TimeZone, endpoint: &str) -> Result<Vec<Item>, Error> {
    crate::validate_date(date)?;
    let (start, end) = bounds(date, &zone)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| Error::Codex("could not initialize HTTP client".into()))?;
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut cursor = None;
        let mut cursors = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut items = Vec::new();
        for _ in 0..100 {
            let mut request = client.get(endpoint).query(&[
                ("from", start.to_string()),
                // The API upper bound is inclusive; discard the next midnight below.
                ("to", end.to_string()),
                ("limit", "100".into()),
                ("order", "asc".into()),
            ]);
            if let Some(cursor) = &cursor {
                request = request.query(&[("cursor", cursor)]);
            }
            let response = request
                .send()
                .await
                .map_err(|_| Error::Codex("request failed; try refreshing again".into()))?;
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok());
                return Err(Error::Codex(match retry {
                    Some(seconds) => format!("rate limited; retry after {seconds} seconds"),
                    None => "rate limited; try again later".into(),
                }));
            }
            if !response.status().is_success() {
                return Err(Error::Codex(format!(
                    "request returned HTTP {}",
                    response.status().as_u16()
                )));
            }
            let page: Page = response
                .json()
                .await
                .map_err(|_| Error::Codex("unsupported API response".into()))?;
            for reset in page.data {
                if reset.id.trim().is_empty()
                    || reset.id.len() > 64
                    || !ids.insert(reset.id.clone())
                {
                    return Err(Error::Codex("invalid or repeated announcement ID".into()));
                }
                if let Some(item) = project_reset(reset, start, end, &zone)? {
                    items.push(item);
                }
            }
            if !page.pagination.has_more {
                if page.pagination.next_cursor.is_some() {
                    return Err(Error::Codex("invalid pagination cursor".into()));
                }
                return Ok(items);
            }
            cursor = Some(
                page.pagination
                    .next_cursor
                    .filter(|value| {
                        !value.is_empty()
                            && value.len() <= 1024
                            && value.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                            })
                            && cursors.insert(value.clone())
                    })
                    .ok_or_else(|| Error::Codex("invalid pagination cursor".into()))?,
            );
        }
        Err(Error::Codex("too many announcement pages".into()))
    })
    .await
    .unwrap_or_else(|_| Err(Error::Codex("calendar read timed out".into())))
}

/// Projects one API reset into a calendar item, validating its timestamp and source.
/// Returns `Ok(None)` for the next-midnight boundary, which the inclusive API upper
/// bound already covers with the following day.
fn project_reset(
    reset: Reset,
    start: Timestamp,
    end: Timestamp,
    zone: &TimeZone,
) -> Result<Option<Item>, Error> {
    let timestamp: Timestamp = reset
        .announced_at
        .parse()
        .map_err(|_| Error::Codex("announcement has an invalid date".into()))?;
    if timestamp < start || timestamp > end {
        return Err(Error::Codex(
            "announcement is outside the requested day".into(),
        ));
    }
    if timestamp == end {
        return Ok(None);
    }
    let local = timestamp.to_zoned(zone.clone());
    let title = match reset.reset_type {
        ResetType::Regular => "Codex usage reset",
        ResetType::Banked => "Codex reset credit",
    };
    let source = match reset.source {
        Source::XPost { url } => Some(url),
        Source::Observed { url } => url,
    };
    let description = match source {
        Some(source) => {
            let url = url::Url::parse(&source)
                .map_err(|_| Error::Codex("announcement has an invalid source URL".into()))?;
            if !matches!(url.scheme(), "https" | "http") {
                return Err(Error::Codex(
                    "announcement has an invalid source URL".into(),
                ));
            }
            format!("{}\n\nSource: {source}", reset.text)
        }
        None => reset.text,
    };
    let time = format!("{:02}:{:02}", local.hour(), local.minute());
    Ok(Some(Item {
        id: format!("codex:{}", reset.id),
        text: format!("{time} {title}"),
        description: Some(description),
        completed: false,
        rollover: false,
        details: Some(Details {
            calendar: "Codex Resets".into(),
            start_date: local.date().to_string(),
            start_time: Some(time),
            end_date: None,
            end_time: None,
            location: None,
        }),
    }))
}

fn bounds(date: &str, zone: &TimeZone) -> Result<(Timestamp, Timestamp), Error> {
    let day: Date = date.parse().map_err(|_| Error::InvalidDate(date.into()))?;
    let tomorrow = day.tomorrow().map_err(|_| Error::DateOverflow)?;
    let start = day
        .at(0, 0, 0, 0)
        .to_zoned(zone.clone())
        .map_err(|_| Error::Codex("could not determine the local day".into()))?;
    let end = tomorrow
        .at(0, 0, 0, 0)
        .to_zoned(zone.clone())
        .map_err(|_| Error::Codex("could not determine the next local day".into()))?;
    Ok((start.timestamp(), end.timestamp()))
}

#[cfg(test)]
#[path = "../tests/unit/codex.rs"]
mod tests;
