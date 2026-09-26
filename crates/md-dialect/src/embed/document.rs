use super::{EmbedError, align, diff, escape_html, fields, is_web_url, reject_unknown, required};
use url::Url;

pub(super) fn render(kind: &str, source: &str) -> Result<String, EmbedError> {
    let normalized = source.replace("\r\n", "\n");
    let mut lines = normalized.split('\n');
    let mut metadata = Vec::new();
    let mut separator = false;
    for line in lines.by_ref() {
        if line == "---" {
            separator = true;
            break;
        }
        metadata.push(line);
    }
    if !separator {
        return Err(EmbedError::InvalidDocument(
            "expected metadata, then --- and a body",
        ));
    }
    let body = lines.collect::<Vec<_>>().join("\n");
    if body.trim().is_empty() {
        return Err(EmbedError::InvalidDocument("body must not be empty"));
    }
    let metadata = metadata.join("\n");
    let mut values = fields(kind, &metadata)?;
    let allowed: &[&str] = if kind == "embed:annotation" {
        &["mark", "note", "color", "url", "align"]
    } else if kind == "embed:quote" {
        &["author", "title", "url", "align"]
    } else {
        &["title", "align"]
    };
    reject_unknown(kind, &values, allowed)?;
    let align = align(&mut values)?;
    if kind == "embed:diff" {
        let title = escape_html(required(&mut values, "diff", "title")?);
        let content = diff::render(&body)?;
        return Ok(format!(
            "<figure class=\"content-embed content-embed-diff content-embed-{align}\"><figcaption>{title}</figcaption><pre tabindex=\"0\" role=\"region\" aria-label=\"{title}\"><code>{content}</code></pre></figure>\n"
        ));
    }
    if kind == "embed:annotation" {
        let mark = required(&mut values, "annotation", "mark")?;
        let note = escape_html(required(&mut values, "annotation", "note")?);
        let color = values.remove("color").unwrap_or("blue");
        if !matches!(color, "blue" | "red" | "green" | "amber" | "purple") {
            return Err(EmbedError::InvalidDocument("invalid annotation color"));
        }
        let Some(start) = body
            .find(mark)
            .filter(|start| Some(*start) == body.rfind(mark))
        else {
            return Err(EmbedError::InvalidDocument(
                "annotation mark must occur exactly once",
            ));
        };
        let note = match values.remove("url") {
            Some(value) => format!(
                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{note}</a>",
                source_url(value)?
            ),
            None => note,
        };
        return Ok(format!(
            "<figure class=\"content-embed content-embed-annotation content-embed-{align} annotation-{color}\"><p>{}<mark>{}</mark>{}</p><figcaption>{note}</figcaption></figure>\n",
            escape_html(&body[..start]),
            escape_html(mark),
            escape_html(&body[start + mark.len()..])
        ));
    }
    let author = escape_html(required(&mut values, "quote", "author")?);
    let title = values
        .remove("title")
        .map(|text| format!(" · <cite>{}</cite>", escape_html(text)))
        .unwrap_or_default();
    let mut cite = String::new();
    let mut link = String::new();
    if let Some(value) = values.remove("url") {
        let url = source_url(value)?;
        cite = format!(" cite=\"{url}\"");
        link =
            format!(" · <a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\">{url}</a>");
    }
    Ok(format!(
        "<figure class=\"content-embed content-embed-quote content-embed-{align}\"><blockquote{cite}><p>{}</p></blockquote><figcaption>{author}{title}{link}</figcaption></figure>\n",
        escape_html(&body)
    ))
}

fn source_url(value: &str) -> Result<String, EmbedError> {
    let invalid = || {
        EmbedError::InvalidDocument("source URL must use HTTP(S) without credentials or whitespace")
    };
    if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid());
    }
    let url = Url::parse(value).map_err(|_| invalid())?;
    if !is_web_url(&url) {
        return Err(invalid());
    }
    Ok(escape_html(url.as_str()))
}
