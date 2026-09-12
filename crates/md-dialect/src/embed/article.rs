use super::{ArticleMetadata, Data, EmbedError, escape_html, reject_unknown};
use std::collections::HashMap;

pub(super) struct Article<'a> {
    pub id: Option<&'a str>,
    pub destination: String,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
}

pub(super) fn parse<'a>(mut fields: HashMap<&str, &'a str>) -> Result<Article<'a>, EmbedError> {
    reject_unknown(
        "embed:article",
        &fields,
        &["title", "id", "url", "description"],
    )?;
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
        id,
        destination,
        title: fields.remove("title"),
        description: fields.remove("description"),
    })
}

pub(super) fn render(fields: HashMap<&str, &str>, data: &Data) -> Result<String, EmbedError> {
    let item = parse(fields)?;
    let key = item.id.unwrap_or(&item.destination);
    let metadata = data.articles.get(key);
    let title = item
        .title
        .or_else(|| metadata.map(|item| item.title.as_str()))
        .unwrap_or("Article unavailable");
    let description = item
        .description
        .or_else(|| metadata.map(|item: &ArticleMetadata| item.description.as_str()));
    let description = description
        .filter(|value| !value.is_empty())
        .map(|description| {
            format!(
                "<span class=\"content-embed-description\">{}</span>",
                escape_html(description)
            )
        })
        .unwrap_or_default();
    let href = metadata
        .and_then(|item| item.href.as_deref())
        .unwrap_or(&item.destination);
    let target = if item.id.is_some() || href.starts_with("/articles/") {
        ""
    } else {
        " target=\"_blank\" rel=\"noopener noreferrer\""
    };
    let unavailable = if item.title.is_none() && metadata.is_none() {
        "<span class=\"content-embed-description\">Preview unavailable</span>"
    } else {
        ""
    };
    Ok(format!(
        "<a class=\"content-embed content-embed-article content-embed-wide\" href=\"{}\"{target}><span class=\"content-article-icon\" aria-hidden=\"true\"></span><span class=\"content-embed-copy\"><strong>{}</strong>{description}{unavailable}</span></a>\n",
        escape_html(href),
        escape_html(title),
    ))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 240
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// URL-only lines belong to explicit article-list fences, never ordinary Markdown links.
pub(super) fn urls(source: &str) -> Result<Option<Vec<String>>, EmbedError> {
    let source = source.trim();
    if source
        .split_once(':')
        .is_some_and(|(field, _)| ["id", "url", "title", "description"].contains(&field.trim()))
    {
        return Ok(None);
    }
    let lines: Vec<_> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() || lines.len() > 50 {
        return Err(EmbedError::InvalidArticleList);
    }
    let urls = lines
        .into_iter()
        .map(|line| {
            let value = match line.as_bytes() {
                [b'-' | b'*' | b'+', space, ..] if space.is_ascii_whitespace() => {
                    line[2..].trim_start()
                }
                _ => line,
            };
            if value.chars().any(char::is_whitespace) {
                return Err(EmbedError::InvalidArticleUrl);
            }
            parse(HashMap::from([("url", value)])).map(|article| article.destination)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(urls))
}

pub(super) fn render_source(source: &str, data: &Data) -> Result<String, EmbedError> {
    let Some(urls) = urls(source)? else {
        return render(super::fields("embed:article", source)?, data);
    };
    let mut html = String::from("<ul class=\"content-article-list\">\n");
    for url in urls {
        html.push_str("<li>");
        html.push_str(&render(HashMap::from([("url", url.as_str())]), data)?);
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n");
    Ok(html)
}
