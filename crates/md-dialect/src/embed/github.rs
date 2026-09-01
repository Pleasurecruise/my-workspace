use super::{Data, EmbedError, escape_html, reject_unknown, required};
use std::collections::HashMap;

pub(super) fn render(mut fields: HashMap<&str, &str>, data: &Data) -> Result<String, EmbedError> {
    reject_unknown("embed:github", &fields, &["repo", "align"])?;
    let repo = required(&mut fields, "github", "repo")?;
    if !valid(repo) {
        return Err(EmbedError::InvalidRepository(repo.to_owned()));
    }
    let align = fields.remove("align").unwrap_or("wide");
    match align {
        "left" | "right" | "wide" => {}
        value => return Err(EmbedError::InvalidAlignment(value.to_owned())),
    }
    let item = match data.repositories.get(repo) {
        Some(item) => item,
        None => {
            return Err(EmbedError::MissingData {
                kind: "github",
                id: repo.to_owned(),
            });
        }
    };
    let description = escape_html(&item.description);
    let language = escape_html(&item.language);
    let name = escape_html(&item.full_name);
    let url = escape_html(&item.url);
    let avatar = escape_html(&item.owner_avatar_url);
    Ok(format!(
        concat!(
            "<a class=\"content-embed content-embed-github content-embed-{align}\" href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\" aria-label=\"GitHub repository {name}\">",
            "<span class=\"content-embed-copy\"><span class=\"content-embed-label\">",
            "<svg class=\"content-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"18\" r=\"3\"/><circle cx=\"6\" cy=\"6\" r=\"3\"/><circle cx=\"18\" cy=\"6\" r=\"3\"/><path d=\"M18 9a9 9 0 0 1-9 9\"/><path d=\"M6 9a9 9 0 0 0 9 9\"/></svg>Repository</span>",
            "<strong>{name}</strong><span class=\"content-embed-description\">{description}</span><span class=\"content-embed-meta\"><span>{language}</span>",
            "<span><svg class=\"content-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.12 2.12 0 0 0 1.595 1.16l5.166.75a.53.53 0 0 1 .294.904l-3.738 3.643a2.12 2.12 0 0 0-.611 1.878l.882 5.146a.53.53 0 0 1-.77.559l-4.62-2.428a2.12 2.12 0 0 0-1.973 0l-4.62 2.428a.53.53 0 0 1-.77-.559l.882-5.146a2.12 2.12 0 0 0-.611-1.878L2.16 9.788a.53.53 0 0 1 .294-.904l5.166-.75a2.12 2.12 0 0 0 1.595-1.16z\"/></svg>{stars}</span>",
            "<span><svg class=\"content-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"18\" r=\"3\"/><circle cx=\"6\" cy=\"6\" r=\"3\"/><circle cx=\"18\" cy=\"6\" r=\"3\"/><path d=\"M18 9a9 9 0 0 1-9 9\"/><path d=\"M6 9a9 9 0 0 0 9 9\"/></svg>{forks}</span>",
            "<span><svg class=\"content-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"1\"/><circle cx=\"12\" cy=\"12\" r=\"10\"/></svg>{issues}</span>",
            "</span></span><img class=\"content-embed-avatar\" src=\"{avatar}\" alt=\"\" loading=\"lazy\" referrerpolicy=\"no-referrer\" /></a>\n"
        ),
        align = align,
        url = url,
        name = name,
        description = description,
        language = language,
        avatar = avatar,
        stars = item.stars,
        forks = item.forks,
        issues = item.open_issues,
    ))
}

pub(super) fn valid(value: &str) -> bool {
    let mut parts = value.split('/');
    let Some(owner) = parts.next() else {
        return false;
    };
    let Some(repo) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    if !part(owner, 39) {
        return false;
    }
    part(repo, 100)
}

fn part(value: &str, max: usize) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.len() > max {
        return false;
    }
    value.bytes().all(|byte| match byte {
        b'-' | b'_' | b'.' => true,
        _ => byte.is_ascii_alphanumeric(),
    })
}
