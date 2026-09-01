mod architecture;
mod canvas;
mod github;
mod stock;
mod storyboard;
mod style;
#[cfg(test)]
mod tests;

use futures_util::stream::{self, StreamExt, TryStreamExt};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::collections::{HashMap, HashSet};

const GITHUB: &str = "embed:github";
const STOCK: &str = "embed:stock";
const ARCHITECTURE: &str = "embed:architecture";
const STORYBOARD: &str = "embed:storyboard";
const DATA_CONCURRENCY: usize = 4;

#[derive(Default)]
pub(crate) struct Data {
    repositories: HashMap<String, quotes::github::RepositorySnapshot>,
    stocks: HashMap<String, quotes::stocks::StockSeries>,
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
    #[error("invalid stock code `{0}`")]
    InvalidStockCode(String),
    #[error("could not resolve embed data: {0}")]
    Data(String),
    #[error("{kind} embed data for `{id}` was not resolved")]
    MissingData { kind: &'static str, id: String },
    #[error("could not sanitize {kind} SVG canvas: {message}")]
    InvalidCanvas { kind: &'static str, message: String },
}

pub(crate) async fn load(source: &str) -> Result<Data, EmbedError> {
    let mut block: Option<(String, String)> = None;
    let mut repositories = HashSet::new();
    let mut stocks = HashSet::new();
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
            Event::Text(text) if block.is_some() => {
                if let Some((_, source)) = &mut block {
                    source.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) if block.is_some() => {
                let (language, source) = block.take().expect("code block starts before ending");
                match language.as_str() {
                    GITHUB => {
                        let mut parsed = fields(&language, &source)?;
                        let repo = required(&mut parsed, "github", "repo")?;
                        if !github::valid(repo) {
                            return Err(EmbedError::InvalidRepository(repo.to_owned()));
                        }
                        repositories.insert(repo.to_owned());
                    }
                    STOCK => {
                        let mut parsed = fields(&language, &source)?;
                        let code = required(&mut parsed, "stock", "code")?.to_ascii_uppercase();
                        if !stock::valid(&code) {
                            return Err(EmbedError::InvalidStockCode(code));
                        }
                        stocks.insert(code);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let mut data = Data::default();
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

pub fn render(language: &str, source: &str, data: &Data) -> Result<Option<String>, EmbedError> {
    if !language.starts_with("embed:") {
        return Ok(None);
    }
    match language {
        GITHUB => github::render(fields(language, source)?, data).map(Some),
        STOCK => stock::render(fields(language, source)?, data).map(Some),
        ARCHITECTURE => architecture::render(source).map(Some),
        STORYBOARD => storyboard::render(source).map(Some),
        kind => Err(EmbedError::UnsupportedKind(kind.to_owned())),
    }
}

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
