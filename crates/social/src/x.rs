mod auth;

pub use auth::{authenticate, authorization};

use super::text::render_x;
use super::{MemoPublication, PublicationProvider, PublishError, PublishedPost, memo_url};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vesper_credentials::Stored;

const ENDPOINT: &str = "https://api.x.com/2/tweets";
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Serialize)]
struct Request<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct Response {
    data: Post,
}

#[derive(Deserialize)]
struct Post {
    id: String,
}

pub async fn publish(memo: &MemoPublication) -> Result<PublishedPost, PublishError> {
    let memo_url = memo_url(memo)?;
    let mut credentials = match vesper_credentials::x()? {
        Stored::Ready(credentials) => credentials,
        Stored::Missing => return Err(PublishError::MissingCredentials("X")),
    };
    if auth::expires_soon(&credentials) {
        credentials = auth::refresh(&credentials).await?;
        vesper_credentials::save_x(credentials.clone())?;
    }
    let text = render_x(&memo.content, &memo_url);
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("vesper/1.0")
        .build()
        .map_err(|_| PublishError::Request("X"))?;
    let response = client
        .post(ENDPOINT)
        .bearer_auth(credentials.access_token)
        .json(&Request { text: &text })
        .send()
        .await
        .map_err(|_| PublishError::Request("X"))?;
    if response.status() != StatusCode::CREATED {
        return Err(PublishError::Status {
            provider: "X",
            status: response.status(),
        });
    }
    let result: Response = response
        .json()
        .await
        .map_err(|_| PublishError::Protocol("X"))?;
    if result.data.id.is_empty()
        || !result
            .data
            .id
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(PublishError::Protocol("X"));
    }
    Ok(PublishedPost {
        provider: PublicationProvider::X,
        url: Some(format!("https://x.com/i/web/status/{}", result.data.id)),
        external_id: result.data.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_response() {
        let x: Response = serde_json::from_str(r#"{"data":{"id":"12345","text":"post"}}"#).unwrap();
        assert_eq!(x.data.id, "12345");
    }
}
