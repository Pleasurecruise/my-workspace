pub mod credentials;
pub mod graph;
pub mod memories;

use serde::de::DeserializeOwned;
use std::time::Duration;
use vault::Stored;

const TIMEOUT: Duration = Duration::from_secs(20);

fn present(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn get<T: DeserializeOwned>(path: &str, query: &[(&str, String)]) -> Result<T, String> {
    let Stored::Ready(credentials) = credentials::read().map_err(|error| error.to_string())? else {
        return Err(
            "Nowledge Mem is not configured; add its server URL and API key in Settings".to_owned(),
        );
    };
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("Vesper/", env!("CARGO_PKG_VERSION"), " nowledge"))
        .build()
        .map_err(|error| format!("Could not create Nowledge Mem client: {error}"))?;
    let response = client
        .get(format!("{}{path}", credentials.url))
        .query(query)
        .bearer_auth(&credentials.api_key)
        .send()
        .await
        .map_err(|error| format!("Could not reach Nowledge Mem: {}", error.without_url()))?;
    if !response.status().is_success() {
        return Err(format!(
            "Nowledge Mem request {path} failed: HTTP {}",
            response.status()
        ));
    }
    response
        .json()
        .await
        .map_err(|error| format!("Nowledge Mem returned an unsupported {path} payload: {error}"))
}
