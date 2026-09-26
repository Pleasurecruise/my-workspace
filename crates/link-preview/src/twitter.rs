//! Public X / Twitter cards, without account credentials or executable embeds.
use serde::Deserialize;
use std::time::Duration;
use url::Url;

/// Fields consumed by the shared Markdown renderer, from Twitter's syndication response.
#[derive(Debug, Deserialize)]
pub struct Post {
    #[serde(rename = "id_str")]
    pub id: String,
    pub text: String,
    pub user: Author,
    pub created_at: String,
    #[serde(default)]
    pub entities: Entities,
    pub display_text_range: [usize; 2],
    #[serde(rename = "mediaDetails", default)]
    pub media: Vec<Media>,
    pub quoted_tweet: Option<Box<Post>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Entities {
    #[serde(default)]
    pub urls: Vec<Link>,
    #[serde(default)]
    pub user_mentions: Vec<Mention>,
    #[serde(default)]
    pub hashtags: Vec<Hashtag>,
}

#[derive(Debug, Deserialize)]
pub struct Link {
    pub indices: [usize; 2],
    pub expanded_url: String,
    pub display_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Mention {
    pub indices: [usize; 2],
    pub screen_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Hashtag {
    pub indices: [usize; 2],
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub name: String,
    pub screen_name: String,
    pub profile_image_url_https: String,
}

#[derive(Debug, Deserialize)]
pub struct Media {
    pub media_url_https: String,
    pub ext_alt_text: Option<String>,
    pub video_info: Option<Video>,
}

#[derive(Debug, Deserialize)]
pub struct Video {
    pub variants: Vec<Variant>,
}

#[derive(Debug, Deserialize)]
pub struct Variant {
    pub content_type: String,
    pub url: String,
    pub bitrate: Option<u64>,
}

pub fn canonical_url(value: &str) -> Result<String, String> {
    let invalid = || "Twitter embed requires an HTTPS X / Twitter status URL".to_owned();
    if value
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err(invalid());
    }
    let url = Url::parse(value).map_err(|_| invalid())?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || !matches!(
            url.host_str(),
            Some("x.com" | "www.x.com" | "twitter.com" | "www.twitter.com" | "mobile.twitter.com")
        )
    {
        return Err(invalid());
    }
    let path = url.path().strip_suffix('/').unwrap_or(url.path());
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() != 4 || !parts[0].is_empty() || parts[2] != "status" {
        return Err(invalid());
    }
    let (handle, id) = (parts[1], parts[3]);
    if !(1..=15).contains(&handle.len())
        || !handle
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        || !(1..=20).contains(&id.len())
        || id.starts_with('0')
        || !id.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(invalid());
    }
    Ok(format!(
        "https://x.com/{}/status/{id}",
        handle.to_ascii_lowercase()
    ))
}

pub async fn read(value: &str) -> Result<Post, String> {
    let url = canonical_url(value)?;
    let id = url
        .rsplit('/')
        .next()
        .expect("canonical status URL has an ID");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "could not create Twitter preview client")?;
    let mut response = client
        .get("https://cdn.syndication.twimg.com/tweet-result")
        .query(&[("id", id), ("lang", "en"), ("token", "1")])
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, "Vesper")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("could not fetch Twitter preview: {error}"))?;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "could not read Twitter preview")?
    {
        if body.len() + chunk.len() > super::MAX_HTML_BYTES {
            return Err("Twitter preview exceeds 1 MiB".to_owned());
        }
        body.extend_from_slice(&chunk);
    }
    let post = parse(&body)?;
    if post.id != id {
        return Err("Twitter preview returned a different post".to_owned());
    }
    Ok(post)
}

fn parse(body: &[u8]) -> Result<Post, String> {
    let post: Post = serde_json::from_slice(body).map_err(|_| "invalid Twitter preview")?;
    if post.user.name.trim().is_empty() || post.text.trim().is_empty() {
        return Err("Twitter preview is missing author or body".to_owned());
    }
    Ok(post)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_supported_hosts() {
        for host in [
            "x.com",
            "www.x.com",
            "twitter.com",
            "www.twitter.com",
            "mobile.twitter.com",
        ] {
            assert_eq!(
                canonical_url(&format!("https://{host}/Example/status/12345/?s=20#media")).unwrap(),
                "https://x.com/example/status/12345"
            );
        }
    }

    #[test]
    fn rejects_non_status_and_unsafe_urls() {
        for url in [
            "https://x.com/example",
            "https://x.com/a/status/no",
            "https://x.com.evil/a/status/1",
            "http://x.com/a/status/1",
            "https://user@x.com/a/status/1",
            "https://x.com:444/a/status/1",
            "https://x.com/a/status/1/photo/1",
            "https://x.com/a/status/0",
            "https://x.com/a/status/1\n",
            "https://x.com\\a/status/1",
        ] {
            assert!(canonical_url(url).is_err(), "{url}");
        }
    }

    #[test]
    fn reads_syndication_media() {
        let post = parse(include_bytes!("../tests/fixtures/tweet.json")).unwrap();
        assert_eq!(post.user.screen_name, "example");
        assert_eq!(post.media.len(), 1);
        assert!(post.quoted_tweet.is_some());
        assert!(parse(br#"{}"#).is_err());
        assert!(parse(br#"{"__typename":"TweetTombstone"}"#).is_err());
    }
}
