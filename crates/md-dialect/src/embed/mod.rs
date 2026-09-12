mod architecture;
mod article;
mod canvas;
mod github;
mod link;
mod media;
mod stock;
mod storyboard;
mod style;
#[cfg(test)]
mod tests;

use futures_util::stream::{self, StreamExt, TryStreamExt};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::collections::{HashMap, HashSet};

const GITHUB: &str = "embed:github";
const ARTICLE: &str = "embed:article";
const LINK: &str = "embed:link";
const MEDIA: &str = "embed:media";
const STOCK: &str = "embed:stock";
const ARCHITECTURE: &str = "embed:architecture";
const STORYBOARD: &str = "embed:storyboard";
const DATA_CONCURRENCY: usize = 4;

/// Resolved provider snapshots used by embed rendering. Fields remain provider-owned.
#[derive(Default)]
pub struct Data {
    repositories: HashMap<String, quotes::github::RepositorySnapshot>,
    links: HashMap<String, quotes::opengraph::Metadata>,
    stocks: HashMap<String, quotes::stocks::StockSeries>,
    pub articles: HashMap<String, ArticleMetadata>,
}

/// Metadata resolved by the authorized host or a public webpage preview.
pub struct ArticleMetadata {
    pub href: Option<String>,
    pub title: String,
    pub description: String,
}

/// Internal references that need metadata, deduplicated in document order.
pub fn article_ids(source: &str) -> Result<Vec<String>, EmbedError> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for (language, source) in parse_fences(source) {
        if language != ARTICLE {
            continue;
        }
        if article::urls(&source)?.is_some() {
            continue;
        }
        let article = article::parse(fields(&language, &source)?)?;
        let Some(id) = article.id else {
            continue;
        };
        if article.title.is_some() && article.description.is_some() {
            continue;
        }
        if seen.insert(id.to_owned()) {
            ids.push(id.to_owned());
        }
    }
    Ok(ids)
}

/// URL references are scoped to explicit article fences and preserve document order.
pub fn article_urls(source: &str) -> Result<Vec<String>, EmbedError> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();
    for (language, source) in parse_fences(source) {
        if language != ARTICLE {
            continue;
        }
        let entries = match article::urls(&source)? {
            Some(urls) => urls,
            None => {
                let item = article::parse(fields(&language, &source)?)?;
                if item.id.is_some() {
                    continue;
                }
                vec![item.destination]
            }
        };
        for url in entries {
            if seen.insert(url.clone()) {
                urls.push(url);
            }
        }
    }
    Ok(urls)
}

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("unsupported embed kind `{0}`")]
    UnsupportedKind(String),
    #[error("{kind} embed line {line} must use `field: value`")]
    InvalidLine { kind: String, line: usize },
    #[error("{kind} embed contains duplicate field `{field}`")]
    DuplicateField { kind: String, field: String },
    #[error("{kind} embed does not support field `{field}`")]
    UnknownField { kind: String, field: String },
    #[error("{kind} embed requires field `{field}`")]
    MissingField {
        kind: &'static str,
        field: &'static str,
    },
    #[error("invalid GitHub repository `{0}`; expected `owner/name`")]
    InvalidRepository(String),
    #[error("invalid embed alignment `{0}`; expected `left`, `right`, or `wide`")]
    InvalidAlignment(String),
    #[error("invalid media field `{field}`: {message}")]
    InvalidMedia {
        field: &'static str,
        message: &'static str,
    },
    #[error(
        "article URL must be an absolute HTTP(S) address without credentials or control characters"
    )]
    InvalidArticleUrl,
    #[error(
        "article embed requires exactly one of `id` or `url`; IDs use up to 240 ASCII letters, digits, hyphens, or underscores"
    )]
    InvalidArticleTarget,
    #[error("article lists require 1–50 URLs")]
    InvalidArticleList,
    #[error("invalid stock code `{0}`")]
    InvalidStockCode(String),
    #[error("could not resolve embed data: {0}")]
    Data(String),
    #[error("{kind} embed data for `{id}` was not resolved")]
    MissingData { kind: &'static str, id: String },
    #[error("could not sanitize {kind} SVG canvas: {message}")]
    InvalidCanvas { kind: &'static str, message: String },
}

/// Resolve provider data referenced by namespaced fences in the document.
pub async fn load(source: &str) -> Result<Data, EmbedError> {
    load_with_articles(source, HashMap::new()).await
}

/// Host-resolved article metadata takes precedence over anonymous website previews.
pub async fn load_with_articles(
    source: &str,
    articles: HashMap<String, ArticleMetadata>,
) -> Result<Data, EmbedError> {
    let mut repositories = HashSet::new();
    let mut stocks = HashSet::new();
    let mut links = HashSet::new();
    let mut article_urls = HashSet::new();
    for (language, source) in parse_fences(source) {
        match language.as_str() {
            GITHUB => {
                let mut parsed = fields(&language, &source)?;
                let repo = required(&mut parsed, "github", "repo")?;
                if !github::valid(repo) {
                    return Err(EmbedError::InvalidRepository(repo.to_owned()));
                }
                repositories.insert(repo.to_owned());
            }
            LINK => {
                let parsed = fields(&language, &source)?;
                let (url, _) = link::parse(parsed)?;
                links.insert(url.to_owned());
            }
            STOCK => {
                let mut parsed = fields(&language, &source)?;
                let code = required(&mut parsed, "stock", "code")?.to_ascii_uppercase();
                if !stock::valid(&code) {
                    return Err(EmbedError::InvalidStockCode(code));
                }
                stocks.insert(code);
            }
            ARTICLE => {
                if let Some(urls) = article::urls(&source)? {
                    article_urls.extend(urls.into_iter().filter(|url| !articles.contains_key(url)));
                    continue;
                }
                let item = article::parse(fields(&language, &source)?)?;
                if item.id.is_none()
                    && !articles.contains_key(&item.destination)
                    && (item.title.is_none() || item.description.is_none())
                {
                    article_urls.insert(item.destination);
                }
            }
            MEDIA => {
                media::parse(fields(&language, &source)?)?;
            }
            _ => {}
        }
    }

    let mut data = Data {
        articles,
        ..Data::default()
    };
    let repository_data = stream::iter(repositories.into_iter().map(|repo| async move {
        let snapshot = quotes::github::read_repository(&repo).await?;
        Ok::<_, String>((repo, snapshot))
    }))
    .buffer_unordered(DATA_CONCURRENCY)
    .try_collect::<Vec<_>>()
    .await
    .map_err(EmbedError::Data)?;
    for (repo, snapshot) in repository_data {
        data.repositories.insert(repo, snapshot);
    }
    data.links = stream::iter(links.into_iter().map(|url| async move {
        let metadata = quotes::opengraph::read(&url).await?;
        Ok::<_, String>((url, metadata))
    }))
    .buffer_unordered(DATA_CONCURRENCY)
    .try_collect()
    .await
    .map_err(EmbedError::Data)?;
    let previews: HashMap<_, _> = stream::iter(article_urls.into_iter().map(|url| async move {
        let metadata = quotes::opengraph::read(&url).await.ok()?;
        Some((
            url,
            ArticleMetadata {
                href: None,
                title: metadata.title,
                description: metadata.description,
            },
        ))
    }))
    .buffer_unordered(DATA_CONCURRENCY)
    .filter_map(async |item| item)
    .collect()
    .await;
    data.articles.extend(previews);
    if !stocks.is_empty() {
        let report = quotes::stocks::read(stocks.into_iter().collect())
            .await
            .map_err(EmbedError::Data)?;
        if let Some(failure) = report.failures.into_iter().next() {
            return Err(EmbedError::Data(failure.message));
        }
        for stock in report.stocks {
            data.stocks.insert(stock.symbol.clone(), stock);
        }
    }
    Ok(data)
}

/// Render a namespaced fence, or return `None` for ordinary code languages.
pub fn render(language: &str, source: &str, data: &Data) -> Result<Option<String>, EmbedError> {
    if !language.starts_with("embed:") {
        return Ok(None);
    }
    match language {
        GITHUB => github::render(fields(language, source)?, data).map(Some),
        ARTICLE => article::render_source(source, data).map(Some),
        LINK => link::render(fields(language, source)?, data).map(Some),
        MEDIA => media::render(fields(language, source)?).map(Some),
        STOCK => stock::render(fields(language, source)?, data).map(Some),
        ARCHITECTURE => architecture::render(source).map(Some),
        STORYBOARD => storyboard::render(source).map(Some),
        kind => Err(EmbedError::UnsupportedKind(kind.to_owned())),
    }
}

/// Add the shared embed stylesheet once when assembling a document.
pub fn add_styles(html: &mut String) {
    if html.contains("class=\"content-embed ") {
        html.insert_str(0, style::CSS);
    }
}

fn fields<'a>(kind: &str, source: &'a str) -> Result<HashMap<&'a str, &'a str>, EmbedError> {
    let mut fields = HashMap::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let Some((field, value)) = line.split_once(':') else {
            return Err(EmbedError::InvalidLine {
                kind: kind.to_owned(),
                line: index + 1,
            });
        };
        let field = field.trim();
        let value = unquote(value.trim());
        if field.is_empty() {
            return Err(EmbedError::InvalidLine {
                kind: kind.to_owned(),
                line: index + 1,
            });
        }
        if value.is_empty() {
            return Err(EmbedError::InvalidLine {
                kind: kind.to_owned(),
                line: index + 1,
            });
        }
        if fields.insert(field, value).is_some() {
            return Err(EmbedError::DuplicateField {
                kind: kind.to_owned(),
                field: field.to_owned(),
            });
        }
    }
    Ok(fields)
}

fn unquote(value: &str) -> &str {
    if value.len() < 2 {
        return value;
    }
    match (value.as_bytes().first(), value.as_bytes().last()) {
        (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\'')) => &value[1..value.len() - 1],
        _ => value,
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn required<'a>(
    fields: &mut HashMap<&str, &'a str>,
    kind: &'static str,
    field: &'static str,
) -> Result<&'a str, EmbedError> {
    fields
        .remove(field)
        .ok_or(EmbedError::MissingField { kind, field })
}

fn reject_unknown(
    kind: &str,
    fields: &HashMap<&str, &str>,
    allowed: &[&str],
) -> Result<(), EmbedError> {
    if let Some(field) = fields.keys().find(|field| !allowed.contains(field)) {
        return Err(EmbedError::UnknownField {
            kind: kind.to_owned(),
            field: (*field).to_owned(),
        });
    }
    Ok(())
}

/// Collect document-relative media assets for the publication builder to validate and copy.
/// Remote sources are rendered directly and never downloaded during compilation.
pub fn collect_media_paths(source: &str) -> Result<Vec<String>, EmbedError> {
    let mut paths = Vec::new();
    for (language, source) in parse_fences(source) {
        if language != MEDIA {
            continue;
        }
        let item = media::parse(fields(&language, &source)?)?;
        for source in [Some(item.src), item.poster].into_iter().flatten() {
            if source.local {
                paths.push(source.url);
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn parse_fences(source: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut block: Option<(String, String)> = None;
    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let language = info
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                block = Some((language, String::new()));
            }
            Event::Text(text) => {
                if let Some((_, source)) = &mut block {
                    source.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = block.take() {
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }
    blocks
}
