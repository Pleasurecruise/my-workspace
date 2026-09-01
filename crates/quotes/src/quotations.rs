use serde::{Deserialize, Serialize};
use std::time::Duration;

const ENDPOINT: &str = "https://api.freeapi.app/api/v1/public/quotes/quote/random";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quotation {
    pub id: u64,
    pub content: String,
    pub author: String,
    pub author_slug: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
struct Envelope {
    success: bool,
    data: QuotationWire,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuotationWire {
    id: u64,
    content: String,
    author: String,
    author_slug: String,
    #[serde(default)]
    tags: Vec<String>,
}

fn project(envelope: Envelope) -> Result<Quotation, String> {
    if !envelope.success {
        return Err("Random quotation provider reported a failure".to_owned());
    }
    let content = envelope.data.content.trim();
    let author = envelope.data.author.trim();
    let author_slug = envelope.data.author_slug.trim();
    if content.is_empty() || author.is_empty() || author_slug.is_empty() {
        return Err("Random quotation provider returned incomplete data".to_owned());
    }
    Ok(Quotation {
        id: envelope.data.id,
        content: content.to_owned(),
        author: author.to_owned(),
        author_slug: author_slug.to_owned(),
        tags: envelope
            .data
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_owned())
            .filter(|tag| !tag.is_empty())
            .collect(),
    })
}

pub async fn read() -> Result<Quotation, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Vesper/0.1 random quotation")
        .build()
        .map_err(|error| format!("Could not create random quotation client: {error}"))?;
    let response = client
        .get(ENDPOINT)
        .send()
        .await
        .map_err(|error| format!("Could not query random quotation: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Random quotation request failed: HTTP {}",
            response.status()
        ));
    }
    let envelope = response.json().await.map_err(|error| {
        format!("Random quotation provider returned an unsupported payload: {error}")
    })?;
    project(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(content: &str) -> Envelope {
        serde_json::from_value(serde_json::json!({
            "success": true,
            "data": {
                "id": 195,
                "content": content,
                "author": "Peter Drucker",
                "authorSlug": "peter-drucker",
                "tags": ["Business", ""]
            }
        }))
        .expect("valid quotation response")
    }

    #[test]
    fn projects_quotation() {
        let quotation =
            project(payload("Change creates opportunity.")).expect("valid quotation projection");

        assert_eq!(quotation.id, 195);
        assert_eq!(quotation.content, "Change creates opportunity.");
        assert_eq!(quotation.author, "Peter Drucker");
        assert_eq!(quotation.tags, ["Business"]);
    }

    #[test]
    fn rejects_empty_content() {
        let error = project(payload("  ")).expect_err("empty quotation should fail");

        assert!(error.contains("incomplete"));
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn reads_live_quotation() {
        let quotation = read().await.expect("quotation should be readable");

        assert!(!quotation.content.is_empty());
        assert!(!quotation.author.is_empty());
    }
}
