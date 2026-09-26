use super::{Data, EmbedError, align, escape_html, is_web_url, reject_unknown, required};
use link_preview::twitter::Post;
use std::collections::HashMap;
use std::fmt::Write;
use time::{OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description};

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
    let post = match post {
        Ok(post) => post,
        Err(error) => {
            return Ok(format!(
                "<aside class=\"content-embed content-embed-twitter content-embed-{align}\"><p>{}</p><a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">Open X / Twitter to read the post.</a></aside>\n",
                escape_html(error),
                escape_html(&url),
            ));
        }
    };
    Ok(format!(
        "<div class=\"content-embed content-embed-twitter content-embed-{align}\">{}</div>\n",
        card(post, false)?
    ))
}

// Main and quoted posts share the same provider contract and safe HTML rendering.
fn card(post: &Post, quoted: bool) -> Result<String, EmbedError> {
    let url = link_preview::twitter::canonical_url(&format!(
        "https://x.com/{}/status/{}",
        post.user.screen_name, post.id
    ))
    .map_err(EmbedError::Data)?;
    for address in std::iter::once(post.user.profile_image_url_https.as_str())
        .chain(
            post.media
                .iter()
                .map(|media| media.media_url_https.as_str()),
        )
        .chain(
            post.media
                .iter()
                .filter_map(|media| media.video_info.as_ref())
                .flat_map(|video| video.variants.iter().map(|variant| variant.url.as_str())),
        )
    {
        if !url::Url::parse(address).is_ok_and(|url| is_web_url(&url)) {
            return Err(EmbedError::Data(
                "Twitter preview contains an invalid media URL".to_owned(),
            ));
        }
    }
    let date = OffsetDateTime::parse(&post.created_at, &Rfc3339)
        .map_err(|_| EmbedError::Data("Twitter preview contains an invalid date".to_owned()))?;
    let date = date
        .format(format_description!(
            "[hour]:[minute] UTC · [month repr:short] [day], [year]"
        ))
        .map_err(|error| EmbedError::Data(error.to_string()))?;
    let text: Vec<_> = post.text.chars().collect();
    let [start, end] = post.display_text_range;
    if start > end || end > text.len() {
        return Err(EmbedError::Data(
            "Twitter preview contains an invalid text range".to_owned(),
        ));
    }
    let mut entities: Vec<_> = post
        .entities
        .urls
        .iter()
        .map(|link| {
            (
                link.indices,
                link.expanded_url.clone(),
                link.display_url.clone(),
            )
        })
        .collect();
    entities.extend(post.entities.user_mentions.iter().map(|mention| {
        (
            mention.indices,
            format!("https://x.com/{}", mention.screen_name),
            format!("@{}", mention.screen_name),
        )
    }));
    entities.extend(post.entities.hashtags.iter().map(|tag| {
        let mut address = url::Url::parse("https://x.com/hashtag/").unwrap();
        address
            .path_segments_mut()
            .unwrap()
            .pop_if_empty()
            .push(&tag.text);
        (tag.indices, address.to_string(), format!("#{}", tag.text))
    }));
    entities.sort_by_key(|(range, _, _)| range[0]);
    let mut body = String::new();
    let mut offset = start;
    for ([from, to], address, label) in entities {
        if from < offset || to > end || from >= to {
            continue;
        }
        body.push_str(&escape_html(&html_escape::decode_html_entities(
            &text[offset..from].iter().collect::<String>(),
        )));
        if url::Url::parse(&address).is_ok_and(|url| is_web_url(&url)) {
            write!(
                body,
                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                escape_html(&address),
                escape_html(&label)
            )
            .unwrap();
        } else {
            body.push_str(&escape_html(&html_escape::decode_html_entities(
                &text[from..to].iter().collect::<String>(),
            )));
        }
        offset = to;
    }
    body.push_str(&escape_html(&html_escape::decode_html_entities(
        &text[offset..end].iter().collect::<String>(),
    )));
    let mut html = format!(
        "<article class=\"tweet-post{}\"><header class=\"tweet-header\"><a class=\"tweet-author\" href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\"><img src=\"{}\" alt=\"\" loading=\"lazy\" referrerpolicy=\"no-referrer\" /><span><strong>{}</strong><span>@{}</span></span></a><a class=\"tweet-brand\" href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\" aria-label=\"View post on X\">𝕏</a></header><p class=\"tweet-body\">{}</p>",
        if quoted { " tweet-quote" } else { "" },
        escape_html(&post.user.profile_image_url_https),
        escape_html(&post.user.name),
        escape_html(&post.user.screen_name),
        body,
    );
    if !post.media.is_empty() {
        html.push_str("<div class=\"tweet-media\">");
        for media in &post.media {
            let image = escape_html(&media.media_url_https);
            let alt = escape_html(media.ext_alt_text.as_deref().unwrap_or("Post media"));
            if let Some(video) = &media.video_info {
                let variant = video
                    .variants
                    .iter()
                    .filter(|variant| variant.content_type == "video/mp4")
                    .max_by_key(|variant| variant.bitrate);
                if let Some(variant) = variant {
                    write!(html, "<figure class=\"content-embed-media tweet-video\"><video controls preload=\"metadata\" playsinline poster=\"{image}\" aria-label=\"{alt}\"><source src=\"{}\" type=\"video/mp4\" /></video></figure>", escape_html(&variant.url)).unwrap();
                } else {
                    write!(html, "<a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\"><img src=\"{image}\" alt=\"{alt}\" loading=\"lazy\" referrerpolicy=\"no-referrer\" /><span>Watch on X</span></a>").unwrap();
                }
            } else {
                write!(html, "<a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\"><img src=\"{image}\" alt=\"{alt}\" loading=\"lazy\" referrerpolicy=\"no-referrer\" /></a>").unwrap();
            }
        }
        html.push_str("</div>");
    }
    if !quoted && let Some(quote) = &post.quoted_tweet {
        html.push_str(&card(quote, true)?);
    }
    write!(html, "<footer class=\"tweet-date\"><a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\"><time datetime=\"{}\">{}</time></a></footer></article>", escape_html(&post.created_at), escape_html(&date)).unwrap();
    Ok(html)
}
