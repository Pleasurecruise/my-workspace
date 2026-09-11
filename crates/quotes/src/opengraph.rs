use dom_query::Document;
use reqwest::{Url, dns};
use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Duration};

const MAX_HTML_BYTES: usize = 1024 * 1024;

pub struct Metadata {
    pub url: String,
    pub title: String,
    pub description: String,
    pub site_name: String,
    pub image: Option<String>,
}

/// Link cards read public HTTP(S) pages without credentials or a browser session.
pub fn validate_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "invalid link URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err("link URL must use HTTP(S) without credentials".to_owned());
    }
    match url.host() {
        Some(url::Host::Ipv4(ip)) if !public_ip(ip.into()) => {
            return Err("link URL must use a public address".to_owned());
        }
        Some(url::Host::Ipv6(ip)) if !public_ip(ip.into()) => {
            return Err("link URL must use a public address".to_owned());
        }
        _ => {}
    }
    Ok(url)
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_documentation()
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && b == 0 && c == 0)
                && !(a == 198 && (b == 18 || b == 19))
        }
        IpAddr::V6(ip) => {
            // Limit IPv6 to global unicast; exclude transition and documentation ranges.
            let segments = ip.segments();
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] < 0x200 || segments[1] == 0xdb8))
                && segments[0] != 0x2002
                && !(segments[0] == 0x3fff && segments[1] < 0x1000)
        }
    }
}

struct PublicDns;

impl dns::Resolve for PublicDns {
    fn resolve(&self, name: dns::Name) -> dns::Resolving {
        Box::pin(async move {
            let addresses: Vec<_> = tokio::net::lookup_host((name.as_str(), 0)).await?.collect();
            if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
                return Err(
                    std::io::Error::other("link host must resolve to public addresses").into(),
                );
            }
            Ok(Box::new(addresses.into_iter()) as dns::Addrs)
        })
    }
}

pub async fn read(value: &str) -> Result<Metadata, String> {
    let url = validate_url(value)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .no_proxy()
        .dns_resolver(Arc::new(PublicDns))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 5 || validate_url(attempt.url().as_str()).is_err() {
                attempt.error("link redirect is not allowed")
            } else {
                attempt.follow()
            }
        }))
        .user_agent("Vesper/1.0 (Open Graph link preview)")
        .build()
        .map_err(|_| "could not create link metadata client")?;
    let mut response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "text/html, application/xhtml+xml")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "could not fetch link metadata")?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    if !content_type.eq_ignore_ascii_case("text/html")
        && !content_type.eq_ignore_ascii_case("application/xhtml+xml")
    {
        return Err("link response is not HTML".to_owned());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_HTML_BYTES as u64)
    {
        return Err("link HTML exceeds 1 MiB".to_owned());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "could not read link HTML")?
    {
        if body.len() + chunk.len() > MAX_HTML_BYTES {
            return Err("link HTML exceeds 1 MiB".to_owned());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(parse(&String::from_utf8_lossy(&body), response.url()))
}

fn parse(html: &str, url: &Url) -> Metadata {
    let document = Document::from(html);
    let mut tags = HashMap::new();
    for meta in document.select("head meta").iter() {
        let Some(name) = meta.attr("property").or_else(|| meta.attr("name")) else {
            continue;
        };
        let Some(value) = meta.attr("content") else {
            continue;
        };
        let value = value.trim();
        if !value.is_empty() {
            tags.entry(name.to_ascii_lowercase())
                .or_insert_with(|| value.to_owned());
        }
    }
    let title = tags.remove("og:title").unwrap_or_else(|| {
        document
            .select_single("head title")
            .text()
            .trim()
            .to_owned()
    });
    let image = tags
        .remove("og:image")
        .and_then(|value| url.join(&value).ok())
        .filter(|url| validate_url(url.as_str()).is_ok())
        .map(String::from);
    Metadata {
        url: url.to_string(),
        title: if title.is_empty() {
            url.host_str().unwrap_or_default().to_owned()
        } else {
            title
        },
        description: tags
            .remove("og:description")
            .or_else(|| tags.remove("description"))
            .unwrap_or_default(),
        site_name: tags
            .remove("og:site_name")
            .unwrap_or_else(|| url.host_str().unwrap_or_default().to_owned()),
        image,
    }
}

#[cfg(test)]
#[path = "../tests/unit/opengraph.rs"]
mod tests;
