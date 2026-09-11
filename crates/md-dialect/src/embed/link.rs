use super::{Data, EmbedError, escape_html, reject_unknown, required};
use std::collections::HashMap;

pub(super) fn parse<'a>(
    mut fields: HashMap<&str, &'a str>,
) -> Result<(&'a str, &'a str), EmbedError> {
    reject_unknown("embed:link", &fields, &["url", "align"])?;
    let url = required(&mut fields, "link", "url")?;
    quotes::opengraph::validate_url(url).map_err(EmbedError::Data)?;
    let align = fields.remove("align").unwrap_or("wide");
    if !matches!(align, "left" | "right" | "wide") {
        return Err(EmbedError::InvalidAlignment(align.to_owned()));
    }
    Ok((url, align))
}

pub(super) fn render(fields: HashMap<&str, &str>, data: &Data) -> Result<String, EmbedError> {
    let (url, align) = parse(fields)?;
    let item = data.links.get(url).ok_or_else(|| EmbedError::MissingData {
        kind: "link",
        id: url.to_owned(),
    })?;
    let image = item.image.as_ref().map(|image| format!(
        "<img class=\"content-embed-thumbnail\" src=\"{}\" alt=\"\" loading=\"lazy\" referrerpolicy=\"no-referrer\" />", escape_html(image)
    )).unwrap_or_default();
    Ok(format!(
        concat!(
            "<a class=\"content-embed content-embed-link content-embed-{align}\" href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\">",
            "<span class=\"content-embed-copy\"><span class=\"content-embed-label\">{site}</span>",
            "<strong>{title}</strong><span class=\"content-embed-description\">{description}</span></span>{image}</a>\n"
        ),
        align = align,
        image = image,
        url = escape_html(&item.url),
        site = escape_html(&item.site_name),
        title = escape_html(&item.title),
        description = escape_html(&item.description),
    ))
}
