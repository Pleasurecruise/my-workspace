use crate::{Details, Error, Item};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use vesper_credentials::NotionCalendar;

#[derive(Deserialize)]
struct View {
    name: String,
    data_source_id: uuid::Uuid,
    configuration: Option<Calendar>,
}

#[derive(Deserialize)]
struct Calendar {
    date_property_id: Option<String>,
}

#[derive(Deserialize)]
struct DataSource {
    properties: BTreeMap<String, Field>,
}

#[derive(Deserialize)]
struct Field {
    id: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Query {
    id: uuid::Uuid,
    #[serde(flatten)]
    page: QueryPage<PageReference>,
}

#[derive(Deserialize)]
struct QueryPage<T> {
    results: Vec<T>,
    request_status: Option<QueryStatus>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct QueryStatus {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct PageReference {
    id: uuid::Uuid,
}

#[derive(Deserialize)]
struct Page {
    id: String,
    properties: BTreeMap<String, Property>,
}

#[derive(Deserialize)]
struct Property {
    id: String,
    #[serde(flatten)]
    value: PropertyValue,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PropertyValue {
    Title {
        title: Vec<Text>,
    },
    Date {
        date: Option<DateRange>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Text {
    plain_text: String,
}

#[derive(Deserialize)]
struct DateRange {
    start: String,
    end: Option<String>,
    time_zone: Option<String>,
}

pub(crate) async fn read(configuration: &NotionCalendar) -> Result<Vec<Item>, Error> {
    let view_id = configuration.view_id()?;
    let binary = binary()?;
    let view_path = format!("/v1/views/{view_id}");
    let view: View = request(&binary, "GET", &view_path, None).await?;
    let property = match view
        .configuration
        .and_then(|calendar| calendar.date_property_id)
    {
        Some(property) => property,
        None => {
            let source: DataSource = request(
                &binary,
                "GET",
                &format!("/v1/data_sources/{}", view.data_source_id),
                None,
            )
            .await?;
            let mut dates = source
                .properties
                .into_values()
                .filter(|property| property.kind == "date");
            let property = dates
                .next()
                .ok_or_else(|| Error::Notion("view has no Date property".into()))?;
            if dates.next().is_some() {
                return Err(Error::Notion(
                    "view has multiple Date properties; use a calendar view to select one".into(),
                ));
            }
            property.id
        }
    };
    let query: Query = request(
        &binary,
        "POST",
        &format!("{view_path}/queries"),
        Some(&serde_json::json!({"page_size": 100})),
    )
    .await?;
    // Notion expires abandoned temporary queries if the read is cancelled.
    let query_path = format!("{view_path}/queries/{}", query.id);
    let mut page = query.page;
    let result = tokio::time::timeout(Duration::from_secs(90), async {
        let mut references = Vec::new();
        let mut cursors = BTreeSet::new();
        loop {
            if page
                .request_status
                .as_ref()
                .is_some_and(|status| status.kind != "complete")
            {
                return Err(Error::Notion(
                    "calendar query was truncated; narrow the view filter".into(),
                ));
            }
            references.extend(
                page.results
                    .into_iter()
                    .map(|reference| reference.id.to_string()),
            );
            if !page.has_more {
                break;
            }
            let cursor = page
                .next_cursor
                .filter(|cursor| cursors.insert(cursor.clone()))
                .ok_or_else(|| Error::Notion("invalid pagination cursor".into()))?;
            let mut path = url::Url::parse("https://api.notion.com").expect("static Notion URL");
            path.set_path(&query_path);
            path.query_pairs_mut()
                .append_pair("start_cursor", &cursor)
                .append_pair("page_size", "100");
            page = request(&binary, "GET", &path[url::Position::BeforePath..], None).await?;
        }
        let ids: BTreeSet<_> = references.iter().cloned().collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut remaining = ids.clone();
        let mut items = BTreeMap::new();
        let mut cursor: Option<String> = None;
        cursors.clear();
        loop {
            let mut body = serde_json::json!({
                "page_size": 100,
            });
            if let Some(cursor) = &cursor {
                body["start_cursor"] = serde_json::json!(cursor);
            }
            let batch: QueryPage<Page> = request(
                &binary,
                "POST",
                &format!("/v1/data_sources/{}/query", view.data_source_id),
                Some(&body),
            )
            .await?;
            if batch
                .request_status
                .as_ref()
                .is_some_and(|status| status.kind != "complete")
            {
                return Err(Error::Notion("calendar data query was incomplete".into()));
            }
            for entry in batch.results {
                if !ids.contains(&entry.id) {
                    continue;
                }
                let id = entry.id.clone();
                remaining.remove(&id);
                if let Some(item) = project(entry, &view_id, &view.name, &property)? {
                    items.insert(id, item);
                }
            }
            if remaining.is_empty() || !batch.has_more {
                break;
            }
            cursor = Some(
                batch
                    .next_cursor
                    .filter(|cursor| cursors.insert(cursor.clone()))
                    .ok_or_else(|| Error::Notion("invalid data source pagination cursor".into()))?,
            );
        }
        if !remaining.is_empty() {
            return Err(Error::Notion(
                "calendar changed during the read; refresh again".into(),
            ));
        }
        Ok(references
            .into_iter()
            .filter_map(|id| items.remove(&id))
            .collect())
    })
    .await
    .unwrap_or_else(|_| {
        Err(Error::Notion(
            "calendar read timed out; narrow the view filter".into(),
        ))
    });
    // Deleting this temporary query does not alter the view or any pages.
    let _ = request::<serde::de::IgnoredAny>(&binary, "DELETE", &query_path, None).await;
    result
}

fn binary() -> Result<PathBuf, Error> {
    let name = if cfg!(windows) { "ntn.exe" } else { "ntn" };
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default();
    // Finder-launched apps do not inherit the user's shell PATH.
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local/bin"));
    }
    paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    paths
        .into_iter()
        .map(|path| path.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            Error::Notion("ntn CLI was not found; install it and run `ntn login`".into())
        })
}

async fn request<T: serde::de::DeserializeOwned>(
    binary: &Path,
    method: &str,
    path: &str,
    body: Option<&serde_json::Value>,
) -> Result<T, Error> {
    tokio::time::sleep(Duration::from_millis(350)).await;
    let mut command = Command::new(binary);
    command
        .args(["api", path, "-X", method, "--notion-version", "2026-03-11"])
        .stdin(Stdio::null())
        .kill_on_drop(true);
    if let Some(body) = body {
        command.args(["--data", &body.to_string()]);
    }
    let result = tokio::time::timeout(Duration::from_secs(20), command.output())
        .await
        .map_err(|_| Error::Notion("ntn request timed out".into()))?;
    let output = result.map_err(|_| Error::Notion("could not start ntn CLI".into()))?;
    if !output.status.success() {
        // CLI diagnostics may contain private response bodies; do not forward stderr.
        let message = match output.status.code() {
            Some(4) => "ntn is not authenticated; run `ntn login` in Terminal",
            _ => "ntn request failed; check CLI login and access to the calendar view",
        };
        return Err(Error::Notion(message.into()));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|_| Error::Notion("unsupported ntn response".into()))
}

fn project(
    page: Page,
    view_id: &str,
    calendar: &str,
    date_property: &str,
) -> Result<Option<Item>, Error> {
    let range = match page
        .properties
        .values()
        .find(|property| {
            percent_encoding::percent_decode_str(&property.id)
                .eq(percent_encoding::percent_decode_str(date_property))
        })
        .map(|property| &property.value)
    {
        Some(PropertyValue::Date { date: Some(range) }) => range,
        Some(PropertyValue::Date { date: None }) => return Ok(None),
        _ => {
            return Err(Error::Notion(
                "calendar must use an accessible Date property".into(),
            ));
        }
    };
    let (start_date, start_time) = local_date(&range.start, range.time_zone.as_deref())?;
    let end = range
        .end
        .as_deref()
        .map(|value| local_date(value, range.time_zone.as_deref()))
        .transpose()?;
    let title = page
        .properties
        .values()
        .find_map(|property| match &property.value {
            PropertyValue::Title { title } => Some(
                title
                    .iter()
                    .map(|text| text.plain_text.as_str())
                    .collect::<String>(),
            ),
            _ => None,
        })
        .ok_or_else(|| Error::Notion("calendar entry has no title property".into()))?;
    let text = if title.trim().is_empty() {
        "Untitled".to_owned()
    } else {
        title
    };
    let text = match &start_time {
        Some(time) => format!("{time} {text}"),
        None => text,
    };
    // Imported titles fit the same visible limit as manually entered tasks.
    let text = text.chars().take(crate::MAX_TEXT_LENGTH).collect();
    Ok(Some(Item {
        id: format!("notion:{view_id}:{}", page.id),
        text,
        description: None,
        completed: false,
        rollover: false,
        details: Some(Details {
            calendar: format!("Notion · {calendar}"),
            start_date,
            start_time,
            end_date: end.as_ref().map(|(date, _)| date.clone()),
            end_time: end.and_then(|(_, time)| time),
            location: None,
        }),
    }))
}

fn local_date(value: &str, zone: Option<&str>) -> Result<(String, Option<String>), Error> {
    if value.len() == 10 {
        crate::validate_date(value)?;
        return Ok((value.to_owned(), None));
    }
    let timestamp: jiff::Timestamp = match zone {
        Some(zone) => {
            let date: jiff::civil::DateTime = value
                .parse()
                .map_err(|_| Error::Notion("calendar entry has an invalid local date".into()))?;
            let zone = jiff::tz::TimeZone::get(zone)
                .map_err(|_| Error::Notion("calendar entry has an invalid time zone".into()))?;
            date.to_zoned(zone)
                .map_err(|_| Error::Notion("calendar entry has an ambiguous local date".into()))?
                .timestamp()
        }
        None => value
            .parse()
            .map_err(|_| Error::Notion("calendar entry has an invalid date".into()))?,
    };
    let local = timestamp.to_zoned(jiff::tz::TimeZone::system());
    Ok((
        local.date().to_string(),
        Some(format!("{:02}:{:02}", local.hour(), local.minute())),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn cli_responses() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("ntn");
        std::fs::write(&binary, r#"#!/bin/sh
[ "$1" = api ] && [ "$2" = /v1/test ] && [ "$3" = -X ] && [ "$4" = POST ] && [ "$5" = --notion-version ] && [ "$6" = 2026-03-11 ] && [ "$7" = --data ] && [ "$8" = '{"page_size":100}' ] || exit 2
printf '%s' '{"results":[],"has_more":false,"next_cursor":null}'
"#).unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        let page: QueryPage<Page> = request(
            &binary,
            "POST",
            "/v1/test",
            Some(&serde_json::json!({"page_size":100})),
        )
        .await
        .unwrap();
        assert!(!page.has_more);
        for (script, expected) in [
            (
                "#!/bin/sh\necho private-response >&2\nexit 4\n",
                "ntn is not authenticated",
            ),
            ("#!/bin/sh\nprintf invalid\n", "unsupported ntn response"),
        ] {
            std::fs::write(&binary, script).unwrap();
            let error = request::<serde::de::IgnoredAny>(&binary, "GET", "/v1/test", None)
                .await
                .err()
                .unwrap()
                .to_string();
            assert!(error.contains(expected));
            assert!(!error.contains("private-response"));
        }
    }

    #[test]
    fn encoded_date_property() {
        let page = r#"{"id":"page","properties":{"Name":{"id":"title","type":"title","title":[{"plain_text":"Event"}]},"Date":{"id":"WZ%5CM","type":"date","date":{"start":"2026-09-07","end":null}}}}"#;
        assert!(
            project(
                serde_json::from_str(page).unwrap(),
                "view",
                "Tasks",
                r"WZ\M"
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn projects_calendar_range() {
        let page = r#"{"id":"page","properties":{"Name":{"id":"title","type":"title","title":[{"plain_text":"Conference"}]},"When":{"id":"date","type":"date","date":{"start":"2026-09-07","end":"2026-09-09"}}}}"#;
        let item = project(serde_json::from_str(page).unwrap(), "view", "Work", "date")
            .unwrap()
            .unwrap();
        assert_eq!(item.id, "notion:view:page");
        assert_eq!(item.text, "Conference");
        assert_eq!(
            item.details.unwrap().end_date.as_deref(),
            Some("2026-09-09")
        );
        assert!(local_date("not-a-date", None).is_err());
    }
}
