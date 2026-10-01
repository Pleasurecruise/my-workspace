use crate::{Error, Result};

const ENDPOINT: &str = "https://lrclib.net/api/get";

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsLine {
    pub start_ms: Option<u64>,
    pub text: String,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub lines: Vec<LyricsLine>,
    pub synced: bool,
    pub instrumental: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    instrumental: bool,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
}

pub(crate) async fn read(
    http: &reqwest::Client,
    artist: &str,
    title: &str,
    album: &str,
    duration_ms: u64,
) -> Result<Option<Lyrics>> {
    let response = http
        .get(ENDPOINT)
        .query(&[
            ("artist_name", artist),
            ("track_name", title),
            ("album_name", album),
            ("duration", &(duration_ms / 1_000).to_string()),
        ])
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(Error::Status {
            operation: "read lyrics",
            status: response.status(),
        });
    }
    let response: Response = response.json().await?;
    if response.instrumental {
        return Ok(Some(Lyrics {
            lines: Vec::new(),
            synced: false,
            instrumental: true,
        }));
    }
    if let Some(synced) = response
        .synced_lyrics
        .filter(|value| !value.trim().is_empty())
    {
        let lines = parse_lrc(&synced);
        return Ok(Some(Lyrics {
            synced: !lines.is_empty(),
            lines,
            instrumental: false,
        }));
    }
    Ok(response.plain_lyrics.and_then(|plain| {
        let lines: Vec<LyricsLine> = plain
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| LyricsLine {
                start_ms: None,
                text: line.to_owned(),
            })
            .collect();
        (!lines.is_empty()).then_some(Lyrics {
            lines,
            synced: false,
            instrumental: false,
        })
    }))
}

fn parse_lrc(value: &str) -> Vec<LyricsLine> {
    let mut lines = Vec::new();
    for raw in value.lines() {
        let Some(close) = raw.find(']') else { continue };
        let stamp = raw.get(1..close).unwrap_or_default();
        let Some((minutes, seconds)) = stamp.split_once(':') else {
            continue;
        };
        let Ok(minutes) = minutes.parse::<u64>() else {
            continue;
        };
        let Ok(seconds) = seconds.parse::<f64>() else {
            continue;
        };
        let text = raw[close + 1..].trim();
        if text.is_empty() {
            continue;
        }
        lines.push(LyricsLine {
            start_ms: Some(minutes * 60_000 + (seconds * 1_000.0).round() as u64),
            text: text.to_owned(),
        });
    }
    lines.sort_by_key(|line| line.start_ms);
    lines
}

pub(crate) fn from_lrc(value: &str) -> Option<Lyrics> {
    let lines = parse_lrc(value);
    (!lines.is_empty()).then_some(Lyrics {
        lines,
        synced: true,
        instrumental: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{from_lrc, parse_lrc};

    #[test]
    fn parses_and_orders_timed_lines() {
        let lines = parse_lrc("[00:10.50]Second\n[00:02.00]First");
        assert_eq!(lines[0].start_ms, Some(2_000));
        assert_eq!(lines[1].text, "Second");
        assert_eq!(lines[1].start_ms, Some(10_500));
    }

    #[test]
    fn skips_metadata_blank_and_malformed_lines() {
        let source = "[ar:Artist]\n[00:01.00]\n[xx:01.00]Bad minutes\n[00:aa]Bad seconds\nno stamp\n[01:02.345]  Kept  ";
        let lines = parse_lrc(source);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Kept");
        assert_eq!(lines[0].start_ms, Some(62_345));
        assert!(from_lrc("[ar:Artist]").is_none());
        assert!(from_lrc("[00:01.00]Line").is_some_and(|lyrics| lyrics.synced));
    }
}
