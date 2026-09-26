//! Public X / Twitter cards, without account credentials or executable embeds.
use dom_query::Document;
use serde::Deserialize;
use std::time::Duration;
use url::Url;

pub struct Post {
    pub author: String,
    pub text: String,
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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "could not create Twitter preview client")?;
    let mut response = client
        .get("https://publish.x.com/oembed")
        .query(&[
            ("url", url.as_str()),
            ("omit_script", "true"),
            ("dnt", "true"),
            ("hide_thread", "true"),
        ])
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "could not fetch Twitter preview")?;
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
    parse(&body)
}

fn parse(body: &[u8]) -> Result<Post, String> {
    #[derive(Deserialize)]
    struct Response {
        author_name: String,
        html: String,
    }
    let response: Response = serde_json::from_slice(body).map_err(|_| "invalid Twitter preview")?;
    let document = Document::from(response.html);
    let paragraph = document.select_single("blockquote > p");
    paragraph.select("script, style").remove();
    paragraph.select("br").replace_with_html("\n");
    let text = paragraph.text().trim().to_owned();
    if response.author_name.trim().is_empty() || text.is_empty() {
        return Err("Twitter preview is missing author or body".to_owned());
    }
    Ok(Post {
        author: response.author_name,
        text,
    })
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
    fn extracts_text_without_provider_scripts() {
        let body = serde_json::json!({"author_name":"Alice", "html":"<blockquote><p>Hello &amp; <a href='x'>world</a><br>next<script>bad()</script><style>bad</style></p>footer</blockquote><script>remote()</script>"});
        let post = parse(&serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(post.author, "Alice");
        assert_eq!(post.text, "Hello & world\nnext");
        assert!(parse(br#"{"author_name":"Alice","html":"<script>bad</script>"}"#).is_err());
    }
}
