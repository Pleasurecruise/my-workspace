use reqwest::{StatusCode, Url, header};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://aihot.news/api/v1/dailies/latest";
const FRESHNESS: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Daily {
    pub date: String,
    pub generated_at: String,
    pub url: String,
    pub lead: Option<Lead>,
    pub sections: Vec<Section>,
    pub flashes: Vec<Flash>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lead {
    pub title: String,
    pub paragraph: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub label: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub title: String,
    pub summary: String,
    pub source: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flash {
    pub title: String,
    pub source: String,
    pub url: String,
    pub published_at: String,
}

#[derive(Deserialize)]
struct Envelope {
    report: Report,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    date: String,
    generated_at: String,
    links: ReportLinks,
    lead: Option<ReportLead>,
    sections: Vec<ReportSection>,
    flashes: Vec<ReportFlash>,
}

#[derive(Deserialize)]
struct ReportLinks {
    aihot: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportLead {
    title: String,
    lead_paragraph: String,
}

#[derive(Deserialize)]
struct ReportSection {
    label: String,
    items: Vec<ReportItem>,
}

#[derive(Deserialize)]
struct ReportItem {
    title: String,
    summary: String,
    source: Source,
    links: Links,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportFlash {
    title: String,
    source: Source,
    links: Links,
    published_at: String,
}

#[derive(Deserialize)]
struct Source {
    name: String,
}

#[derive(Deserialize)]
struct Links {
    original: String,
}

struct Cached {
    etag: Option<String>,
    daily: Daily,
    checked_at: Instant,
}

/// Reads the latest AIHOT daily, revalidating with its ETag at most once per cache window.
#[derive(Default)]
pub struct Reader {
    cache: tokio::sync::Mutex<Option<Cached>>,
}

impl Reader {
    pub async fn read(&self) -> Result<Daily, String> {
        let mut cache = self.cache.lock().await;
        if let Some(cached) = cache.as_ref()
            && cached.checked_at.elapsed() < FRESHNESS
        {
            return Ok(cached.daily.clone());
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("Vesper/", env!("CARGO_PKG_VERSION"), " newspaper"))
            .build()
            .map_err(|error| format!("Could not create the newspaper client: {error}"))?;
        let mut request = client.get(ENDPOINT);
        if let Some(etag) = cache.as_ref().and_then(|cached| cached.etag.as_deref()) {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("Could not reach AIHOT: {error}"))?;
        match response.status() {
            StatusCode::NOT_MODIFIED => {
                let cached = cache
                    .as_mut()
                    .ok_or_else(|| "AIHOT revalidated a daily that was never read".to_owned())?;
                cached.checked_at = Instant::now();
                return Ok(cached.daily.clone());
            }
            StatusCode::TOO_MANY_REQUESTS => {
                return Err("AIHOT rate limit reached. Try again in a minute.".to_owned());
            }
            status if !status.is_success() => {
                return Err(format!("AIHOT daily request failed: HTTP {status}"));
            }
            _ => {}
        }
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let envelope = response
            .json()
            .await
            .map_err(|error| format!("AIHOT returned an unsupported daily: {error}"))?;
        let daily = project(envelope)?;
        *cache = Some(Cached {
            etag,
            daily: daily.clone(),
            checked_at: Instant::now(),
        });
        Ok(daily)
    }
}

fn web_url(value: &str) -> Option<String> {
    Url::parse(value.trim())
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map(String::from)
}

fn project(envelope: Envelope) -> Result<Daily, String> {
    let report = envelope.report;
    let url = web_url(&report.links.aihot)
        .ok_or_else(|| "AIHOT returned a daily without a valid link".to_owned())?;
    let sections = report
        .sections
        .into_iter()
        .map(|section| Section {
            label: section.label.trim().to_owned(),
            items: section
                .items
                .into_iter()
                .filter_map(|item| {
                    Some(Item {
                        url: web_url(&item.links.original)?,
                        title: item.title.trim().to_owned(),
                        summary: item.summary.trim().to_owned(),
                        source: item.source.name.trim().to_owned(),
                    })
                })
                .collect::<Vec<_>>(),
        })
        .filter(|section| !section.items.is_empty())
        .collect();
    let flashes = report
        .flashes
        .into_iter()
        .filter_map(|flash| {
            Some(Flash {
                url: web_url(&flash.links.original)?,
                title: flash.title.trim().to_owned(),
                source: flash.source.name.trim().to_owned(),
                published_at: flash.published_at,
            })
        })
        .collect();
    Ok(Daily {
        date: report.date,
        generated_at: report.generated_at,
        url,
        lead: report.lead.map(|lead| Lead {
            title: lead.title.trim().to_owned(),
            paragraph: lead.lead_paragraph.trim().to_owned(),
        }),
        sections,
        flashes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(original: &str) -> Envelope {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "report": {
                "date": "2026-10-07",
                "generatedAt": "2026-10-07T00:00:29.119Z",
                "windowStart": "2026-10-06T00:00:00.000Z",
                "windowEnd": "2026-10-07T00:00:00.000Z",
                "links": { "aihot": "https://aihot.news/daily/2026-10-07" },
                "lead": { "title": " Lead ", "leadParagraph": " Paragraph " },
                "sections": [
                    {
                        "label": "模型发布/更新",
                        "items": [{
                            "title": "Model",
                            "summary": "Summary",
                            "source": { "name": "Blog" },
                            "links": { "aihot": null, "original": original }
                        }]
                    }
                ],
                "flashes": [{
                    "title": "Flash",
                    "source": { "name": "RSS" },
                    "links": { "aihot": "https://aihot.news/items/a", "original": "https://example.com/flash" },
                    "publishedAt": "2026-10-06T17:32:00.000Z"
                }]
            }
        }))
        .expect("valid daily response")
    }

    #[test]
    fn projects_daily() {
        let daily = project(envelope("https://example.com/model")).expect("valid daily");

        assert_eq!(daily.url, "https://aihot.news/daily/2026-10-07");
        assert_eq!(
            daily.lead,
            Some(Lead {
                title: "Lead".to_owned(),
                paragraph: "Paragraph".to_owned()
            })
        );
        assert_eq!(daily.sections[0].items[0].url, "https://example.com/model");
        assert_eq!(daily.flashes[0].source, "RSS");
    }

    #[test]
    fn drops_items_without_web_links() {
        let daily = project(envelope("javascript:alert(1)")).expect("valid daily");

        assert!(daily.sections.is_empty());
        assert_eq!(daily.flashes.len(), 1);
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn reads_live_daily() {
        let daily = Reader::default()
            .read()
            .await
            .expect("daily should be readable");

        assert!(!daily.date.is_empty());
    }
}
