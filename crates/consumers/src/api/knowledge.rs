use super::{ApiError, Client, send};
use cms_core::markdown::{
    ArticleMetadata, ReadingStats, TocEntry, article_ids, article_urls, compile_knowledge_plain,
    compile_knowledge_with_articles, knowledge_body,
};
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

pub async fn project_article(article: Article) -> Result<Document, ApiError> {
    let edition = article.editions.get("zh").ok_or_else(|| {
        ApiError::Protocol(format!("article {} has no Chinese edition", article.id))
    })?;
    let source = knowledge_body(&edition.markdown).to_owned();
    let mut metadata = HashMap::new();
    let ids = article_ids(&source).unwrap_or_default();
    let urls: Vec<_> = article_urls(&source)
        .unwrap_or_default()
        .into_iter()
        .filter(|url| article_identity(url).is_some())
        .collect();
    let own = Summary {
        id: article.id.clone(),
        editions: article
            .editions
            .iter()
            .map(|(locale, edition)| {
                (
                    locale.clone(),
                    EditionSummary {
                        title: edition.title.clone(),
                        summary: edition.summary.clone(),
                    },
                )
            })
            .collect(),
        tags: article.tags.clone(),
        visibility: article.visibility,
        content_hash: article.content_hash.clone(),
        created_at: article.created_at.clone(),
        updated_at: article.updated_at.clone(),
    };
    resolve_card_metadata(&own, &ids, &urls, &mut metadata);
    let needs_index = ids.iter().any(|id| !metadata.contains_key(id))
        || urls.iter().any(|url| !metadata.contains_key(url));
    let summaries = if needs_index {
        match reference_summaries().await {
            Ok(summaries) => summaries,
            Err(error) => {
                tracing::warn!(
                    article_id = %article.id,
                    %error,
                    "could not read the article index; embeds keep their own metadata"
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    for summary in summaries {
        resolve_card_metadata(&summary, &ids, &urls, &mut metadata);
    }
    let compiled = match compile_knowledge_with_articles(&source, metadata).await {
        Ok(compiled) => compiled,
        Err(error) => {
            tracing::warn!(
                article_id = %article.id,
                %error,
                "could not enrich Knowledge embeds; preserving them as code blocks"
            );
            compile_knowledge_plain(&source)
        }
    };
    let newspaper_edition = newspaper_edition(&article.tags);
    Ok(Document {
        id: article.id,
        title: edition.title.clone(),
        summary: edition.summary.clone(),
        tags: article.tags,
        visibility: article.visibility,
        content_hash: article.content_hash,
        created_at: article.created_at,
        updated_at: article.updated_at,
        newspaper_edition,
        source,
        html: compiled.html,
        toc: compiled.toc,
        stats: compiled.stats,
    })
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

fn resolve_card_metadata(
    summary: &Summary,
    ids: &[String],
    urls: &[String],
    metadata: &mut HashMap<String, ArticleMetadata>,
) {
    let Some(edition) = summary.editions.get("zh") else {
        return;
    };
    for key in ids.iter().filter(|id| *id == &summary.id).chain(
        urls.iter()
            .filter(|url| article_identity(url).is_some_and(|identity| identity == summary.id)),
    ) {
        metadata.insert(
            key.clone(),
            ArticleMetadata {
                href: Some(format!(
                    "/articles/{}{}",
                    summary.id,
                    reqwest::Url::parse(key)
                        .ok()
                        .and_then(|url| url.fragment().map(|fragment| format!("#{fragment}")))
                        .unwrap_or_default()
                )),
                title: edition.title.clone(),
                description: edition.summary.clone(),
            },
        );
    }
}

async fn reference_summaries() -> Result<Vec<Summary>, ApiError> {
    let client = Client::load(ConsumerApi::Knowledge)?;
    let client_ref = &client;
    reference_pages(|filters| async move { read_summary_page(client_ref, &filters).await }).await
}

async fn reference_pages<F, Fut>(mut read: F) -> Result<Vec<Summary>, ApiError>
where
    F: FnMut(ListFilters) -> Fut,
    Fut: std::future::Future<Output = Result<ArticlePage, ApiError>>,
{
    let mut result = Vec::new();
    let mut ids = HashSet::new();
    for tags in [vec![], vec!["daily".to_owned()]] {
        let mut filters = ListFilters {
            limit: Some(OVERVIEW_PAGE_SIZE),
            tags,
            ..Default::default()
        };
        let mut cursors = HashSet::new();
        loop {
            let page = read(ListFilters {
                cursor: filters.cursor.clone(),
                limit: filters.limit,
                tags: filters.tags.clone(),
                ..Default::default()
            })
            .await?;
            result.extend(
                page.articles
                    .into_iter()
                    .filter(|summary| ids.insert(summary.id.clone())),
            );
            let Some(cursor) = page.cursor else {
                break;
            };
            if !cursors.insert(cursor.clone()) {
                return Err(ApiError::Protocol(
                    "article pagination repeated a cursor".to_owned(),
                ));
            }
            filters.cursor = Some(cursor);
        }
    }
    Ok(result)
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn decodes_api_contracts() {
        #[derive(Deserialize)]
        struct Responses {
            list: ArticlePage,
            created: ArticleResponse<Article>,
            detail: ArticleResponse<Article>,
            visibility: ArticleResponse<Summary>,
            search: ArticlePage,
        }
        // Captured from the local generated Knowledge Worker, never production data.
        let source = include_str!("../../tests/fixtures/knowledge-contract.json");
        let responses: Responses = serde_json::from_str(source).unwrap();
        assert!(responses.list.cursor.is_none());
        assert_eq!(responses.list.articles.len(), 1);
        let summary = responses.list.articles.into_iter().next().unwrap();
        let visible = responses.visibility.article;
        assert_eq!(
            serde_json::to_value(&summary).unwrap(),
            serde_json::to_value(&visible).unwrap()
        );
        let entry = project_summary(summary).unwrap();
        assert_eq!(entry.id, responses.detail.article.id);
        assert_eq!(entry.title, "Contract article");
        assert!(matches!(entry.visibility, Visibility::Private));
        assert_eq!(entry.content_hash, responses.detail.article.content_hash);
        assert_ne!(entry.updated_at, responses.detail.article.updated_at);
        assert!(responses.created.article.editions.contains_key("en"));
        assert_eq!(responses.detail.article.editions.len(), 1);
        let document = project_article(responses.detail.article).await.unwrap();
        assert!(document.source.contains(":suzume5_01:"));
        assert!(document.html.contains("markdown-emoji"));
        assert_eq!(responses.search.articles.len(), 1);
        let search =
            project_summary(responses.search.articles.into_iter().next().unwrap()).unwrap();
        assert!(matches!(search.visibility, Visibility::Private));
        assert_eq!(search.tags, ["testing/privacy"]);
    }

    #[test]
    fn validates_write_versions() {
        let version = serde_json::json!({
            "expectedHash": "a".repeat(64),
            "expectedUpdatedAt": "2026-09-20T11:00:00.000Z"
        });
        let mut draft = version.clone();
        draft.as_object_mut().unwrap().extend(serde_json::json!({
            "title": "Title", "summary": "Summary", "body": "Body", "tags": [], "visibility": "private"
        }).as_object().unwrap().clone());
        let decoded: DraftUpdate = serde_json::from_value(draft.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), draft);
        draft.as_object_mut().unwrap().remove("expectedUpdatedAt");
        assert!(serde_json::from_value::<DraftUpdate>(draft).is_err());
        let mut documents = version.clone();
        documents["documents"] = serde_json::json!({"zh": "Chinese document"});
        let decoded: DocumentUpdate = serde_json::from_value(documents.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), documents);
        documents
            .as_object_mut()
            .unwrap()
            .remove("expectedUpdatedAt");
        assert!(serde_json::from_value::<DocumentUpdate>(documents).is_err());
        let mut visibility = version;
        visibility["visibility"] = serde_json::json!("public");
        let decoded: VisibilityUpdate = serde_json::from_value(visibility.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), visibility);
        visibility
            .as_object_mut()
            .unwrap()
            .remove("expectedUpdatedAt");
        assert!(serde_json::from_value::<VisibilityUpdate>(visibility).is_err());
    }

    async fn projected_document(id: &str, tags: &[&str], created_at: &str) -> Document {
        project_article(Article {
            id: id.to_owned(),
            editions: HashMap::from([(
                "zh".to_owned(),
                Edition {
                    title: id.to_owned(),
                    summary: id.to_owned(),
                    markdown: format!("# {id}"),
                },
            )]),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            visibility: Visibility::Private,
            content_hash: id.to_owned(),
            created_at: created_at.to_owned(),
            updated_at: created_at.to_owned(),
        })
        .await
        .expect("article should project")
    }

    #[test]
    fn decodes_article_ids() {
        for (path, id) in [
            ("%61lpha", "alpha"),
            ("%e4%b8%ad%e6%96%87", "中文"),
            ("a%23b", "a#b"),
            ("a%2520b", "a%20b"),
        ] {
            assert_eq!(
                article_identity(&format!("https://knowledge.you-find.me/articles/{path}"))
                    .as_deref(),
                Some(id)
            );
        }
        assert!(article_identity("https://knowledge.you-find.me/articles/%FF").is_none());
        assert!(article_identity("https://example.com/articles/alpha").is_none());
    }

    #[tokio::test]
    async fn resolves_uuid_metadata() {
        let id = "11111111-1111-4111-8111-111111111111";
        let source = format!(
            "```embed:article\nhttps://knowledge.you-find.me/articles/{id}\nhttps://knowledge.you-find.me/articles/{id}#section\nhttps://example.com/articles/{id}\n```"
        );
        let document = project_article(Article {
            id: id.to_owned(),
            editions: HashMap::from([(
                "zh".to_owned(),
                Edition {
                    title: "Current title".to_owned(),
                    summary: "Current summary".to_owned(),
                    markdown: source,
                },
            )]),
            tags: vec![],
            visibility: Visibility::Private,
            content_hash: "hash".to_owned(),
            created_at: "2026-09-13".to_owned(),
            updated_at: "2026-09-13".to_owned(),
        })
        .await
        .unwrap();
        assert_eq!(
            document
                .html
                .matches(&format!("href=\"/articles/{id}\""))
                .count(),
            1
        );
        assert_eq!(
            document
                .html
                .matches("<strong>Current title</strong>")
                .count(),
            2
        );
        assert!(
            document
                .html
                .contains(&format!("href=\"/articles/{id}#section\""))
        );
        assert!(!document.html.contains("href=\"https://example.com"));
    }

    #[tokio::test]
    async fn paginates_reference_index() {
        let mut requests = Vec::new();
        let articles = reference_pages(|filters| {
            requests.push((filters.tags.clone(), filters.cursor.clone()));
            let daily = !filters.tags.is_empty();
            let id = if daily {
                if filters.cursor.is_some() {
                    "older-daily"
                } else {
                    "latest-daily"
                }
            } else {
                "regular"
            };
            std::future::ready(Ok(ArticlePage {
                articles: vec![Summary {
                    id: id.into(),
                    editions: HashMap::new(),
                    tags: filters.tags,
                    visibility: Visibility::Private,
                    content_hash: String::new(),
                    created_at: String::new(),
                    updated_at: String::new(),
                }],
                cursor: if daily && filters.cursor.is_none() {
                    Some("older".into())
                } else {
                    None
                },
            }))
        })
        .await
        .unwrap();
        assert_eq!(
            articles
                .iter()
                .map(|article| article.id.as_str())
                .collect::<Vec<_>>(),
            ["regular", "latest-daily", "older-daily"]
        );
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2], (vec!["daily".into()], Some("older".into())));
        assert!(
            reference_pages(|_| std::future::ready(Ok(ArticlePage {
                articles: vec![],
                cursor: Some("loop".into())
            })))
            .await
            .is_err()
        );
    }

    #[test]
    fn resolves_card_metadata() {
        let summary = Summary {
            id: "real-id".into(),
            editions: HashMap::from([(
                "zh".into(),
                EditionSummary {
                    title: "Actual title".into(),
                    summary: "Actual description".into(),
                },
            )]),
            tags: vec![],
            visibility: Visibility::Private,
            content_hash: "hash".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let url = "https://knowledge.you-find.me/articles/real-id?from=list#section".to_owned();
        let mut metadata = HashMap::new();
        resolve_card_metadata(
            &summary,
            &["real-id".into()],
            std::slice::from_ref(&url),
            &mut metadata,
        );
        assert_eq!(metadata[&url].title, "Actual title");
        assert_eq!(metadata[&url].description, "Actual description");
        assert_eq!(
            metadata[&url].href.as_deref(),
            Some("/articles/real-id#section")
        );
        assert_eq!(
            metadata["real-id"].href.as_deref(),
            Some("/articles/real-id")
        );
    }

    #[tokio::test]
    async fn paginates_overview() {
        let mut requested = Vec::new();
        let summaries = overview_pages(|cursor: Option<String>| {
            requested.push(cursor.clone());
            let (id, next) = match cursor.as_deref() {
                None => ("first", Some("second")),
                Some("second") => ("older", None),
                _ => panic!("unexpected cursor"),
            };
            std::future::ready(Ok(ArticlePage {
                articles: vec![Summary {
                    id: id.to_owned(),
                    editions: HashMap::new(),
                    tags: vec![],
                    visibility: Visibility::Private,
                    content_hash: id.to_owned(),
                    created_at: "2026-09-05T00:00:00Z".to_owned(),
                    updated_at: "2026-09-05T00:00:00Z".to_owned(),
                }],
                cursor: next.map(str::to_owned),
            }))
        })
        .await
        .unwrap();
        assert_eq!(requested, vec![None, Some("second".to_owned())]);
        assert_eq!(
            summaries
                .iter()
                .map(|summary| summary.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "older"]
        );
        let error = overview_pages(|_| async {
            Ok(ArticlePage {
                articles: vec![],
                cursor: Some("same".to_owned()),
            })
        })
        .await
        .expect_err("cursor loop must fail");
        assert!(error.to_string().contains("repeated a cursor"));
        let error = overview_pages(|cursor| async move {
            if cursor.is_some() {
                return Err(ApiError::Protocol("second page failed".to_owned()));
            }
            Ok(ArticlePage {
                articles: vec![],
                cursor: Some("next".to_owned()),
            })
        })
        .await
        .expect_err("failed page must not become a complete overview");
        assert!(error.to_string().contains("second page failed"));
    }

    #[test]
    fn decodes_slugless_summaries() {
        let page: ArticlePage = serde_json::from_value(serde_json::json!({
            "articles": [{
                "id": "019c1234-1234-7000-8000-123456789abc",
                "editions": {
                    "zh": { "title": "类型边界", "summary": "完整的元数据契约" }
                },
                "tags": ["rust", "api"],
                "visibility": "private",
                "contentHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "createdAt": "2026-08-23T10:00:00.000Z",
                "updatedAt": "2026-08-23T11:00:00.000Z"
            }],
            "cursor": "next-page"
        }))
        .expect("valid my-knowledge list response");

        assert_eq!(page.cursor.as_deref(), Some("next-page"));
        assert_eq!(page.articles[0].tags, ["rust", "api"]);
        let summary = &page.articles[0];
        let url = format!(
            "https://knowledge.you-find.me/articles/{}#section",
            summary.id
        );
        let mut metadata = HashMap::new();
        resolve_card_metadata(
            summary,
            std::slice::from_ref(&summary.id),
            std::slice::from_ref(&url),
            &mut metadata,
        );
        assert_eq!(metadata[&url].title, "类型边界");
        assert_eq!(
            metadata[&url].href,
            Some(format!("/articles/{}#section", summary.id))
        );
        assert_eq!(
            metadata[&summary.id].href,
            Some(format!("/articles/{}", summary.id))
        );
        let entry = project_summary(page.articles.into_iter().next().unwrap()).unwrap();
        assert_eq!(entry.title, "类型边界");
        assert!(serde_json::to_value(&entry).unwrap().get("slug").is_none());
    }

    #[test]
    fn projects_overview() {
        fn summary(id: &str, tags: &[&str], created_at: &str) -> Summary {
            Summary {
                id: id.to_owned(),
                editions: HashMap::new(),
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                visibility: Visibility::Private,
                content_hash: id.to_owned(),
                created_at: created_at.to_owned(),
                updated_at: created_at.to_owned(),
            }
        }

        let summaries = vec![
            summary(
                "developer-latest",
                &["developer-daily"],
                "2026-09-02T00:00:00Z",
            ),
            summary("regular-one", &["rust"], "2026-09-01T00:00:00Z"),
            summary(
                "personal-latest",
                &["personal-daily"],
                "2026-08-31T00:00:00Z",
            ),
            summary(
                "developer-old",
                &["developer-daily"],
                "2026-08-30T00:00:00Z",
            ),
            summary("regular-two", &[], "2026-08-29T00:00:00Z"),
            summary("personal-old", &["personal-daily"], "2026-08-28T00:00:00Z"),
        ];
        let ids: Vec<_> = overview_summaries(summaries.clone())
            .into_iter()
            .map(|summary| summary.id)
            .collect();

        assert_eq!(
            ids,
            [
                "developer-latest",
                "regular-one",
                "personal-latest",
                "regular-two",
            ]
        );

        let default_page = summaries
            .iter()
            .filter(|item| item.tags.is_empty() || item.tags == ["rust"]);
        let mut daily: Vec<_> = summaries
            .iter()
            .filter(|item| newspaper_edition(&item.tags).is_some())
            .cloned()
            .collect();
        for item in &mut daily {
            item.tags.push("daily".to_owned());
        }
        daily.reverse();
        daily[0].updated_at = "2026-09-05T00:00:00Z".to_owned();
        let mut retained = Vec::new();
        for page in daily.chunks(2) {
            retained.extend_from_slice(page);
            retained = overview_summaries(retained);
        }
        retained.push(retained[0].clone());
        let merged = overview_summaries(default_page.cloned().chain(retained).collect());
        let ids: Vec<_> = merged.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "regular-one",
                "regular-two",
                "personal-latest",
                "developer-latest"
            ]
        );
    }

    #[test]
    fn decodes_markdown_without_slug() {
        let response: ArticleResponse<Article> = serde_json::from_value(serde_json::json!({
            "article": {
                "id": "019c1234-1234-7000-8000-123456789abc",
                "editions": {
                    "zh": {
                        "title": "类型边界",
                        "summary": "完整的元数据契约",
                        "markdown": "# 类型边界"
                    }
                },
                "tags": ["rust"],
                "visibility": "public",
                "contentHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "createdAt": "2026-08-23T10:00:00.000Z",
                "updatedAt": "2026-08-23T11:00:00.000Z"
            }
        }))
        .expect("valid my-knowledge article response");

        assert_eq!(response.article.editions["zh"].markdown, "# 类型边界");
    }

    #[tokio::test]
    async fn strips_article_header() {
        let article: Article = serde_json::from_value(serde_json::json!({
            "id": "019c1234-1234-7000-8000-123456789abc",
            "editions": {
                    "zh": {
                        "title": "Daily",
                        "summary": "Brief",
                        "markdown": "---\ntitle: Daily\nsummary: Brief\ntags:\n  - newspaper\n  - daily\n---\n## Today\n\nNews\n"
                    }
            },
            "tags": ["newspaper", "daily"],
            "visibility": "public",
            "contentHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "createdAt": "2026-08-24T10:00:00.000Z",
            "updatedAt": "2026-08-24T11:00:00.000Z"
        }))
        .expect("valid my-knowledge article");

        let document = project_article(article)
            .await
            .expect("projected Chinese article");

        assert!(
            serde_json::to_value(&document)
                .unwrap()
                .get("slug")
                .is_none()
        );
        assert_eq!(document.source, "## Today\n\nNews");
        assert!(document.html.starts_with("<h2 id=\"today\">Today</h2>"));
        assert_eq!(document.tags, ["newspaper", "daily"]);
    }

    #[tokio::test]
    async fn retains_failed_embeds() {
        let article: Article = serde_json::from_value(serde_json::json!({
            "id": "019c1234-1234-7000-8000-123456789abc",
            "editions": {
                "zh": {
                    "title": "Unavailable embed",
                    "summary": "The article remains readable",
                    "markdown": "# Article\n\n```embed:github\nrepo: missing-owner\n```"
                }
            },
            "tags": [],
            "visibility": "private",
            "contentHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "createdAt": "2026-08-24T10:00:00.000Z",
            "updatedAt": "2026-08-24T11:00:00.000Z"
        }))
        .expect("valid my-knowledge article");

        let document = project_article(article)
            .await
            .expect("embed failure should not discard the article");

        assert!(document.html.contains("language-embed:github"));
        assert!(document.html.contains("repo: missing-owner"));
    }

    #[tokio::test]
    async fn classifies_news_tags() {
        assert_eq!(
            newspaper_edition(&[" Daily ".to_owned(), "PROGRAMMER-DAILY".to_owned()]),
            Some(NewspaperEdition::Developer)
        );
        assert_eq!(
            newspaper_edition(&["personal-daily".to_owned()]),
            Some(NewspaperEdition::Personal)
        );
        assert_eq!(
            newspaper_edition(&["personal-daily-prompt".to_owned()]),
            None
        );
        assert_eq!(
            newspaper_edition(&["developer-daily".to_owned(), "personal-daily".to_owned()]),
            None
        );

        let document =
            projected_document("developer", &["developer-daily"], "2026-08-25T00:00:00Z").await;
        assert_eq!(
            serde_json::to_value(document).expect("document should serialize")["newspaperEdition"],
            "developer"
        );
    }

    #[tokio::test]
    async fn selects_latest_issues() {
        let documents = [
            projected_document(
                "older-personal",
                &["personal-daily"],
                "2026-08-23T00:00:00Z",
            )
            .await,
            projected_document("developer", &["developer-daily"], "2026-08-25T00:00:00Z").await,
            projected_document("personal", &["personal-daily"], "2026-08-24T00:00:00Z").await,
        ];

        assert_eq!(
            latest_newspaper_issues(&documents.iter().map(Entry::from).collect::<Vec<_>>()),
            NewspaperIssues {
                developer: Some("developer".to_owned()),
                personal: Some("personal".to_owned()),
            }
        );
    }

    #[tokio::test]
    async fn rejects_unsafe_article_ids() {
        let expected_hash = "a".repeat(64);
        let expected_updated_at = "2026-09-20T11:00:00.000Z".to_owned();
        let draft = DraftUpdate {
            expected_hash: expected_hash.clone(),
            expected_updated_at: expected_updated_at.clone(),
            visibility: None,
            title: "Title".to_owned(),
            summary: "Summary".to_owned(),
            body: "Body".to_owned(),
            tags: Vec::new(),
        };
        let documents = DocumentUpdate {
            expected_hash: expected_hash.clone(),
            expected_updated_at: expected_updated_at.clone(),
            documents: Documents {
                zh: "Chinese document".to_owned(),
                en: None,
                ja: None,
            },
        };
        let visibility = VisibilityUpdate {
            expected_hash,
            expected_updated_at,
            visibility: Visibility::Private,
        };
        for id in [
            "",
            "..",
            "../settings",
            "one?token=other",
            "one#fragment",
            "one%2Ftwo",
        ] {
            assert!(matches!(get(id).await, Err(ApiError::Protocol(_))), "{id}");
            assert!(
                matches!(update_draft(id, &draft).await, Err(ApiError::Protocol(_))),
                "{id}"
            );
            assert!(
                matches!(
                    update_documents(id, &documents).await,
                    Err(ApiError::Protocol(_))
                ),
                "{id}"
            );
            assert!(
                matches!(
                    set_visibility(id, &visibility).await,
                    Err(ApiError::Protocol(_))
                ),
                "{id}"
            );
            assert!(
                matches!(
                    delete(id, "hash", "updated").await,
                    Err(ApiError::Protocol(_))
                ),
                "{id}"
            );
        }
    }

    #[tokio::test]
    async fn resolves_self_reference() {
        let document = project_article(Article {
            id: "self".to_owned(),
            editions: HashMap::from([(
                "zh".to_owned(),
                Edition {
                    title: "Automatic title".to_owned(),
                    summary: "Automatic summary".to_owned(),
                    markdown: "```embed:article\nid: self\n```".to_owned(),
                },
            )]),
            tags: Vec::new(),
            visibility: Visibility::Private,
            content_hash: "hash".to_owned(),
            created_at: "2026-09-12".to_owned(),
            updated_at: "2026-09-12".to_owned(),
        })
        .await
        .unwrap();
        assert!(document.html.contains("Automatic title"));
        assert!(document.html.contains("Automatic summary"));
        assert!(!document.html.contains("Preview unavailable"));
    }

    #[tokio::test]
    #[ignore = "manual local index projection benchmark; writes raw samples under /private/tmp"]
    async fn benchmark_index_projection() {
        let markdown = format!("# Sample article\n\n{}", "## Details\n\nA paragraph with **formatting**, [a link](https://example.com), and `code`.\n\n".repeat(100));
        let articles: Vec<_> = (0..100)
            .map(|index| Article {
                id: format!("article-{index}"),
                editions: HashMap::from([(
                    "zh".to_owned(),
                    Edition {
                        title: format!("Article {index}"),
                        summary: "A short article summary".to_owned(),
                        markdown: markdown.clone(),
                    },
                )]),
                tags: vec!["benchmark".to_owned()],
                visibility: Visibility::Private,
                content_hash: format!("hash-{index}"),
                created_at: "2026-09-12T10:00:00Z".to_owned(),
                updated_at: "2026-09-12T10:00:00Z".to_owned(),
            })
            .collect();
        let summaries: Vec<_> = articles
            .iter()
            .map(|article| Summary {
                id: article.id.clone(),
                editions: article
                    .editions
                    .iter()
                    .map(|(locale, edition)| {
                        (
                            locale.clone(),
                            EditionSummary {
                                title: edition.title.clone(),
                                summary: edition.summary.clone(),
                            },
                        )
                    })
                    .collect(),
                tags: article.tags.clone(),
                visibility: article.visibility,
                content_hash: article.content_hash.clone(),
                created_at: article.created_at.clone(),
                updated_at: article.updated_at.clone(),
            })
            .collect();
        let mut candidate = Vec::new();
        for index in 0..18 {
            let started = std::time::Instant::now();
            let entries: Vec<_> = summaries
                .iter()
                .cloned()
                .map(project_summary)
                .collect::<Result<_, _>>()
                .unwrap();
            let serialized = serde_json::to_vec(&entries).unwrap();
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            let value: serde_json::Value = serde_json::from_slice(&serialized).unwrap();
            assert!(value[0].get("source").is_none());
            assert!(value[0].get("html").is_none());
            assert!(value[0].get("toc").is_none());
            if index >= 3 {
                candidate.push(serde_json::json!({ "ms": elapsed, "bytes": serialized.len() }));
            }
        }
        std::fs::write(
            "/private/tmp/vesper-index-candidate.json",
            serde_json::to_vec_pretty(&candidate).unwrap(),
        )
        .unwrap();
        let mut samples = Vec::new();
        for index in 0..18 {
            let started = std::time::Instant::now();
            let mut documents = Vec::new();
            for article in &articles {
                documents.push(project_article(article.clone()).await.unwrap());
            }
            let bytes = serde_json::to_vec(&documents).unwrap().len();
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            if index >= 3 {
                samples.push(serde_json::json!({ "ms": elapsed, "bytes": bytes }));
            }
        }
        std::fs::write(
            "/private/tmp/vesper-index-comparison-full.json",
            serde_json::to_vec_pretty(&samples).unwrap(),
        )
        .unwrap();
    }
}
