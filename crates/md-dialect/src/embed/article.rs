use super::{Data, EmbedError, escape_html, reject_unknown};
use std::collections::HashMap;

pub(super) struct Article<'a> {
    pub id: Option<&'a str>,
    pub destination: String,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub align: &'a str,
}

pub(super) fn parse<'a>(mut fields: HashMap<&str, &'a str>) -> Result<Article<'a>, EmbedError> {
    reject_unknown(
        "embed:article",
        &fields,
        &["title", "id", "url", "description", "align"],
    )?;
    let align = fields.remove("align").unwrap_or("wide");
    if !matches!(align, "left" | "right" | "wide" | "narrow") {
        return Err(EmbedError::InvalidAlignment(align.to_owned()));
    }
    let id = fields.remove("id");
    let destination = match (id, fields.remove("url")) {
        (Some(id), None) if valid_id(id) => format!("/articles/{id}"),
        (None, Some(destination)) => {
            let url = url::Url::parse(destination).map_err(|_| EmbedError::InvalidArticleUrl)?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || destination.chars().any(char::is_control)
            {
                return Err(EmbedError::InvalidArticleUrl);
            }
            url.to_string()
        }
        _ => return Err(EmbedError::InvalidArticleTarget),
    };
    Ok(Article {
        align,
        id,
        destination,
        title: fields.remove("title"),
        description: fields.remove("description"),
    })
}

pub(super) fn render(fields: HashMap<&str, &str>, data: &Data) -> Result<String, EmbedError> {
    let item = parse(fields)?;
    let key = item.id.unwrap_or(&item.destination);
    let Some(metadata) = data.articles.get(key) else {
        return Ok(format!(
            "<div class=\"content-embed content-embed-article content-embed-{}\" aria-disabled=\"true\"><span class=\"content-embed-copy\"><strong>Article unavailable</strong><span class=\"content-embed-description\">Article not found in the article list</span></span></div>\n",
            item.align,
        ));
    };
    let title = item.title.unwrap_or(&metadata.title);
    let description = item.description.or(Some(metadata.description.as_str()));
    let description = description
        .filter(|value| !value.is_empty())
        .map(|description| {
            format!(
                "<span class=\"content-embed-description\">{}</span>",
                escape_html(description)
            )
        })
        .unwrap_or_default();
    let href = metadata.href.as_deref().unwrap_or(&item.destination);
    let target = if item.id.is_some() || href.starts_with("/articles/") {
        ""
    } else {
        " target=\"_blank\" rel=\"noopener noreferrer\""
    };
    Ok(format!(
        "<a class=\"content-embed content-embed-article content-embed-{align}\" href=\"{}\"{target}><span class=\"content-article-icon\" aria-hidden=\"true\"></span><span class=\"content-embed-copy\"><strong>{}</strong>{description}</span></a>\n",
        escape_html(href),
        escape_html(title),
        align = item.align,
    ))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 240
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// A list may carry one alignment field; single-card fields use the normal field parser.
pub(super) fn list(source: &str) -> Result<Option<(&str, Vec<String>)>, EmbedError> {
    let lines: Vec<_> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.iter().any(|line| {
        line.split_once(':')
            .is_some_and(|(field, _)| ["id", "url", "title", "description"].contains(&field.trim()))
    }) {
        return Ok(None);
    }
    let mut align = None;
    let mut urls = Vec::new();
    for line in lines {
        if let Some(("align", value)) = line
            .split_once(':')
            .map(|(key, value)| (key.trim(), value.trim()))
        {
            if align.is_some() {
                return Err(EmbedError::DuplicateField {
                    kind: "embed:article".to_owned(),
                    field: "align".to_owned(),
                });
            }
            let value = super::unquote(value);
            if !matches!(value, "left" | "right" | "wide" | "narrow") {
                return Err(EmbedError::InvalidAlignment(value.to_owned()));
            }
            align = Some(value);
            continue;
        }
        let value = match line.as_bytes() {
            [b'-' | b'*' | b'+', space, ..] if space.is_ascii_whitespace() => {
                line[2..].trim_start()
            }
            _ => line,
        };
        if value.chars().any(char::is_whitespace) {
            return Err(EmbedError::InvalidArticleUrl);
        }
        urls.push(parse(HashMap::from([("url", value)]))?.destination);
    }
    if urls.is_empty() || urls.len() > 50 {
        return Err(EmbedError::InvalidArticleList);
    }
    Ok(Some((align.unwrap_or("wide"), urls)))
}

pub(super) fn render_source(source: &str, data: &Data) -> Result<String, EmbedError> {
    let Some((align, urls)) = list(source)? else {
        return render(super::fields("embed:article", source)?, data);
    };
    let mut html = format!("<ul class=\"content-article-list content-embed-{align}\">\n");
    for url in urls {
        html.push_str("<li>");
        html.push_str(&render(HashMap::from([("url", url.as_str())]), data)?);
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n");
    Ok(html)
}
