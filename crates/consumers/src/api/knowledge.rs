mod render;

pub use render::{preview, project_article};

use super::{ApiError, Client, send};
use cms_core::markdown::{ReadingStats, TocEntry};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use vesper_credentials::ConsumerApi;

const ENDPOINT: &str = "https://knowledge.you-find.me/api/articles";
const OVERVIEW_PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub newspaper_edition: Option<NewspaperEdition>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub newspaper_edition: Option<NewspaperEdition>,
    pub source: String,
    pub html: String,
    pub toc: Vec<TocEntry>,
    pub stats: ReadingStats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NewspaperEdition {
    Developer,
    Personal,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewspaperIssues {
    pub developer: Option<String>,
    pub personal: Option<String>,
}

const DEV_NEWS_TAGS: &[&str] = &[
    "developer-daily",
    "programmer-daily",
    "newspaper/developer",
    "newspaper/developer-daily",
    "newspaper/programmer",
    "newspaper/programmer-daily",
    "程序员日报",
];
const PERSONAL_NEWS_TAGS: &[&str] = &[
    "personal-daily",
    "newspaper/personal",
    "newspaper/personal-daily",
    "个人日报",
    "每日日报",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Edition {
    pub title: String,
    pub summary: String,
    pub markdown: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditionSummary {
    pub title: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub id: String,
    pub editions: HashMap<String, Edition>,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub editions: HashMap<String, EditionSummary>,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub documents: Vec<Entry>,
    pub cursor: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Draft {
    pub title: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Documents {
    pub zh: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ja: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum Create {
    Draft(Draft),
    Documents { documents: Documents },
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftUpdate {
    pub expected_hash: String,
    pub expected_updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentUpdate {
    pub expected_hash: String,
    pub expected_updated_at: String,
    pub documents: Documents,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityUpdate {
    pub expected_hash: String,
    pub expected_updated_at: String,
    pub visibility: Visibility,
}

#[derive(Deserialize, Serialize)]
pub struct ArticlePage {
    pub articles: Vec<Summary>,
    pub cursor: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListFilters {
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub visibility: Option<Visibility>,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub async fn summaries(filters: &ListFilters) -> Result<ArticlePage, ApiError> {
    if filters
        .limit
        .is_some_and(|limit| !(1..=100).contains(&limit))
    {
        return Err(ApiError::Protocol(
            "article limit must be between 1 and 100".to_owned(),
        ));
    }
    if filters.tags.len() > 5 || filters.tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err(ApiError::Protocol(
            "article filters accept at most five non-empty tags".to_owned(),
        ));
    }
    let client = Client::load(ConsumerApi::Knowledge)?;
    read_summary_page(&client, filters).await
}

#[derive(Deserialize)]
struct ArticleResponse<T> {
    article: T,
}

pub async fn list(cursor: Option<String>) -> Result<Page, ApiError> {
    let client = Client::load(ConsumerApi::Knowledge)?;
    let page = read_summary_page(
        &client,
        &ListFilters {
            cursor,
            limit: Some(20),
            ..ListFilters::default()
        },
    )
    .await?;
    let documents = page
        .articles
        .into_iter()
        .map(project_summary)
        .collect::<Result<_, _>>()?;
    Ok(Page {
        documents,
        cursor: page.cursor,
    })
}

pub async fn overview() -> Result<Page, ApiError> {
    let client = Client::load(ConsumerApi::Knowledge)?;
    let client_ref = &client;
    let (regular, daily) = tokio::try_join!(
        overview_pages(|cursor| async move {
            read_summary_page(
                client_ref,
                &ListFilters {
                    cursor,
                    limit: Some(OVERVIEW_PAGE_SIZE),
                    ..ListFilters::default()
                },
            )
            .await
        }),
        overview_pages(|cursor| async move {
            read_summary_page(
                client_ref,
                &ListFilters {
                    cursor,
                    limit: Some(OVERVIEW_PAGE_SIZE),
                    tags: vec!["daily".to_owned()],
                    ..ListFilters::default()
                },
            )
            .await
        }),
    )?;
    let summaries = regular
        .into_iter()
        .chain(
            daily
                .into_iter()
                .filter(|summary| newspaper_edition(&summary.tags).is_some()),
        )
        .collect();
    let documents = overview_summaries(summaries)
        .into_iter()
        .map(project_summary)
        .collect::<Result<_, _>>()?;
    Ok(Page {
        documents,
        cursor: None,
    })
}

async fn overview_pages<F, Fut>(mut read: F) -> Result<Vec<Summary>, ApiError>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: std::future::Future<Output = Result<ArticlePage, ApiError>>,
{
    let mut cursor = None;
    let mut seen_cursors = HashSet::new();
    let mut summaries = Vec::new();
    loop {
        let page = read(cursor).await?;
        summaries.extend(page.articles);
        summaries = overview_summaries(summaries);
        let Some(next) = page.cursor else {
            return Ok(summaries);
        };
        if !seen_cursors.insert(next.clone()) {
            return Err(ApiError::Protocol(
                "article pagination repeated a cursor".to_owned(),
            ));
        }
        cursor = Some(next);
    }
}

async fn read_summary_page(
    client: &Client,
    filters: &ListFilters,
) -> Result<ArticlePage, ApiError> {
    let mut request = client.http.get(ENDPOINT).bearer_auth(&client.api_key);
    if let Some(limit) = filters.limit {
        request = request.query(&[("limit", limit)]);
    }
    if let Some(visibility) = filters.visibility {
        request = request.query(&[("visibility", visibility)]);
    }
    for tag in &filters.tags {
        request = request.query(&[("tag", tag)]);
    }
    if let Some(cursor) = &filters.cursor {
        request = request.query(&[("cursor", cursor)]);
    }
    let response = send(request, "list knowledge articles").await?;
    Ok(response.json().await?)
}

fn project_summary(summary: Summary) -> Result<Entry, ApiError> {
    let edition = summary.editions.get("zh").ok_or_else(|| {
        ApiError::Protocol(format!("article {} has no Chinese summary", summary.id))
    })?;
    let newspaper_edition = newspaper_edition(&summary.tags);
    Ok(Entry {
        id: summary.id,
        title: edition.title.clone(),
        summary: edition.summary.clone(),
        tags: summary.tags,
        visibility: summary.visibility,
        content_hash: summary.content_hash,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        newspaper_edition,
    })
}

impl From<&Document> for Entry {
    fn from(document: &Document) -> Self {
        Self {
            id: document.id.clone(),
            title: document.title.clone(),
            summary: document.summary.clone(),
            tags: document.tags.clone(),
            visibility: document.visibility,
            content_hash: document.content_hash.clone(),
            created_at: document.created_at.clone(),
            updated_at: document.updated_at.clone(),
            newspaper_edition: document.newspaper_edition,
        }
    }
}

fn overview_summaries(summaries: Vec<Summary>) -> Vec<Summary> {
    let mut latest_developer: Option<&Summary> = None;
    let mut latest_personal: Option<&Summary> = None;
    for summary in &summaries {
        let latest = match newspaper_edition(&summary.tags) {
            Some(NewspaperEdition::Developer) => &mut latest_developer,
            Some(NewspaperEdition::Personal) => &mut latest_personal,
            None => continue,
        };
        if latest.is_none_or(|current| is_newer(&summary.created_at, &current.created_at)) {
            *latest = Some(summary);
        }
    }
    let latest_developer = latest_developer.map(|summary| summary.id.clone());
    let latest_personal = latest_personal.map(|summary| summary.id.clone());
    let mut seen = HashSet::new();
    summaries
        .into_iter()
        .filter(|summary| seen.insert(summary.id.clone()))
        .filter(|summary| {
            newspaper_edition(&summary.tags).is_none()
                || Some(&summary.id) == latest_developer.as_ref()
                || Some(&summary.id) == latest_personal.as_ref()
        })
        .collect()
}

fn newspaper_edition(tags: &[String]) -> Option<NewspaperEdition> {
    let mut developer = false;
    let mut personal = false;
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        developer |= DEV_NEWS_TAGS.contains(&tag.as_str());
        personal |= PERSONAL_NEWS_TAGS.contains(&tag.as_str());
    }
    match (developer, personal) {
        (true, false) => Some(NewspaperEdition::Developer),
        (false, true) => Some(NewspaperEdition::Personal),
        _ => None,
    }
}

pub fn latest_newspaper_issues(documents: &[Entry]) -> NewspaperIssues {
    let mut developer: Option<&Entry> = None;
    let mut personal: Option<&Entry> = None;
    for document in documents {
        let issue = match document.newspaper_edition {
            Some(NewspaperEdition::Developer) => &mut developer,
            Some(NewspaperEdition::Personal) => &mut personal,
            None => continue,
        };
        if issue.is_none_or(|current| is_newer(&document.created_at, &current.created_at)) {
            *issue = Some(document);
        }
    }
    NewspaperIssues {
        developer: developer.map(|document| document.id.clone()),
        personal: personal.map(|document| document.id.clone()),
    }
}

/// Compare `created_at` stamps, falling back to string order when either is not RFC 3339.
fn is_newer(candidate: &str, current: &str) -> bool {
    match (
        OffsetDateTime::parse(candidate, &Rfc3339),
        OffsetDateTime::parse(current, &Rfc3339),
    ) {
        (Ok(candidate), Ok(current)) => candidate > current,
        _ => candidate > current,
    }
}

fn article_identity(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    if url.origin().ascii_serialization() != "https://knowledge.you-find.me"
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let id = url.path().strip_prefix("/articles/")?.trim_end_matches('/');
    if id.is_empty() || id.contains('/') {
        return None;
    }
    percent_encoding::percent_decode_str(id)
        .decode_utf8()
        .ok()
        .map(|id| id.into_owned())
}

/// Build the article detail URL, rejecting IDs that could splice the path.
fn build_url(id: &str) -> Result<String, ApiError> {
    if id.is_empty()
        || id.len() > 240
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ApiError::Protocol(
            "invalid knowledge article reference".to_owned(),
        ));
    }
    Ok(format!("{ENDPOINT}/{id}"))
}

pub async fn get(reference: &str) -> Result<Article, ApiError> {
    let id = article_identity(reference).unwrap_or_else(|| reference.to_owned());
    let url = build_url(&id)?;
    let client = Client::load(ConsumerApi::Knowledge)?;
    let response = send(
        client.http.get(url).bearer_auth(&client.api_key),
        "read knowledge article",
    )
    .await?;
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn create(input: &Create) -> Result<Article, ApiError> {
    let client = Client::load(ConsumerApi::Knowledge)?;
    let response = client
        .http
        .post(ENDPOINT)
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::CREATED {
        return Err(mutation_error(response, "create knowledge article").await);
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn update_draft(id: &str, input: &DraftUpdate) -> Result<Article, ApiError> {
    let url = build_url(id)?;
    let client = Client::load(ConsumerApi::Knowledge)?;
    let response = client
        .http
        .patch(url)
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(mutation_error(response, "update knowledge article").await);
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn update_documents(id: &str, input: &DocumentUpdate) -> Result<Article, ApiError> {
    let url = build_url(id)?;
    let client = Client::load(ConsumerApi::Knowledge)?;
    let response = client
        .http
        .patch(url)
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(mutation_error(response, "update knowledge documents").await);
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn set_visibility(id: &str, input: &VisibilityUpdate) -> Result<Summary, ApiError> {
    let url = build_url(id)?;
    let client = Client::load(ConsumerApi::Knowledge)?;
    let response = send(
        client
            .http
            .patch(url)
            .bearer_auth(&client.api_key)
            .json(input),
        "set knowledge visibility",
    )
    .await?;
    let result: ArticleResponse<Summary> = response.json().await?;
    Ok(result.article)
}

pub async fn delete(
    id: &str,
    expected_hash: &str,
    expected_updated_at: &str,
) -> Result<(), ApiError> {
    let url = build_url(id)?;
    let client = Client::load(ConsumerApi::Knowledge)?;
    let response = client
        .http
        .delete(url)
        .bearer_auth(&client.api_key)
        .json(&serde_json::json!({
            "expectedHash": expected_hash,
            "expectedUpdatedAt": expected_updated_at,
        }))
        .send()
        .await?;
    if response.status() == StatusCode::NO_CONTENT {
        Ok(())
    } else {
        Err(ApiError::Status {
            operation: "delete knowledge article",
            status: response.status(),
        })
    }
}

async fn mutation_error(mut response: reqwest::Response, operation: &'static str) -> ApiError {
    let status = response.status();
    let fallback = ApiError::Status { operation, status };
    if status != StatusCode::UNPROCESSABLE_ENTITY {
        return fallback;
    }
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if body.len() + chunk.len() > 8192 {
            return fallback;
        }
        body.extend_from_slice(&chunk);
    }
    #[derive(Deserialize)]
    struct Rejection {
        error: String,
    }
    let Ok(rejection) = serde_json::from_slice::<Rejection>(&body) else {
        return fallback;
    };
    let message = match rejection.error.as_str() {
        "Invalid article update" | "Invalid article input" => "Article fields failed server validation. Check title, summary, tags, Markdown length, and version hash.".to_owned(),
        "Raw HTML is not supported" => "Raw HTML is not supported in Knowledge articles.".to_owned(),
        "Executable URLs are not supported" => "Executable URLs are not supported in Knowledge articles.".to_owned(),
        message if message.starts_with("Unsupported embed kind: ") => {
            let kind = message.trim_start_matches("Unsupported embed kind: ");
            if kind.len() > 48 || !kind.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_')) { return fallback; }
            format!("The Knowledge server does not support {kind}. Update the server dialect before saving this block.")
        }
        _ => "Article content failed server validation. Check tags and structured Markdown blocks.".to_owned(),
    };
    ApiError::Rejected { operation, message }
}

#[cfg(test)]
#[path = "../../tests/unit/knowledge.rs"]
mod tests;
