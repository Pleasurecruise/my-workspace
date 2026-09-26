use super::{Data, EmbedError, align, escape_html, reject_unknown, required};
use std::collections::HashMap;

pub(super) fn parse(mut fields: HashMap<&str, &str>) -> Result<(String, String), EmbedError> {
    reject_unknown("embed:twitter", &fields, &["url", "align"])?;
    let url = required(&mut fields, "twitter", "url")?;
    let url = link_preview::twitter::canonical_url(url).map_err(EmbedError::Data)?;
    Ok((url, align(&mut fields)?.to_owned()))
}

pub(super) fn render(fields: HashMap<&str, &str>, data: &Data) -> Result<String, EmbedError> {
    let (url, align) = parse(fields)?;
    let post = data
        .tweets
        .get(&url)
        .ok_or_else(|| EmbedError::MissingData {
            kind: "twitter",
            id: url.clone(),
        })?;
    let handle = format!("@{}", url.split('/').nth(3).unwrap_or_default());
    let (author, text) = post.as_ref().map_or(
        (
            handle.as_str(),
            "Post preview is unavailable. Open X / Twitter to read the post.",
        ),
        |post| (post.author.as_str(), post.text.as_str()),
    );
    Ok(format!(
        "<a class=\"content-embed content-embed-link content-embed-{align}\" href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"content-embed-copy\"><span class=\"content-embed-label\">X / Twitter</span><strong>{author}</strong><span class=\"content-embed-description content-embed-tweet\">{text}</span></span></a>\n",
        url = escape_html(&url),
        author = escape_html(author),
        text = escape_html(text),
    ))
}
