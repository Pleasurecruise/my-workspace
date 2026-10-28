use serde::{Deserialize, Serialize};
use std::time::Duration;

const ENDPOINT: &str = "https://dummyjson.com/quotes/random";
const TIMEOUT: Duration = Duration::from_secs(15);
const RETRY_DELAY: Duration = Duration::from_millis(300);

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quotation {
    pub id: u64,
    pub content: String,
    pub author: String,
}

#[derive(Deserialize)]
struct QuotationWire {
    id: u64,
    quote: String,
    author: String,
}

fn project(wire: QuotationWire) -> Result<Quotation, String> {
    let content = wire.quote.trim();
    let author = wire.author.trim();
    if content.is_empty() || author.is_empty() {
        return Err("Random quotation provider returned incomplete data".to_owned());
    }
    Ok(Quotation {
        id: wire.id,
        content: content.to_owned(),
        author: author.to_owned(),
    })
}

async fn fetch(client: &reqwest::Client) -> Result<Quotation, String> {
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
    let wire = response.json().await.map_err(|error| {
        format!("Random quotation provider returned an unsupported payload: {error}")
    })?;
    project(wire)
}

pub async fn read() -> Result<Quotation, String> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!(
            "Vesper/",
            env!("CARGO_PKG_VERSION"),
            " random quotation"
        ))
        .build()
        .map_err(|error| format!("Could not create random quotation client: {error}"))?;
    // One retry absorbs a transient timeout or gateway error without hiding an outage.
    match fetch(&client).await {
        Ok(quotation) => Ok(quotation),
        Err(first) => {
            tokio::time::sleep(RETRY_DELAY).await;
            fetch(&client)
                .await
                .map_err(|second| format!("{first}; retry failed: {second}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(quote: &str, author: &str) -> QuotationWire {
        serde_json::from_value(serde_json::json!({
            "id": 195,
            "quote": quote,
            "author": author
        }))
        .expect("valid quotation response")
    }

    #[test]
    fn projects_quotation() {
        let quotation = project(payload(" Change creates opportunity. ", " Peter Drucker "))
            .expect("valid quotation projection");

        assert_eq!(quotation.id, 195);
        assert_eq!(quotation.content, "Change creates opportunity.");
        assert_eq!(quotation.author, "Peter Drucker");
    }

    #[test]
    fn rejects_empty_content() {
        let error = project(payload("  ", "Peter Drucker")).expect_err("empty quote should fail");

        assert!(error.contains("incomplete"));
    }

    #[test]
    fn rejects_empty_author() {
        let error = project(payload("Change creates opportunity.", " ")).expect_err("empty author");

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
