use super::ApiError;
use futures_util::stream::{self, StreamExt, TryStreamExt};
use md_dialect::{TocEntry, compile_knowledge_enriched, compile_knowledge_plain, knowledge_body};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use vesper_credentials::{ConsumerApi, Stored};

const ENDPOINT: &str = "https://knowledge.you-find.me/api/articles";
const READ_CONCURRENCY: usize = 6;
const OVERVIEW_PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub slug: String,
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

#[derive(Clone)]
struct Client {
    api_key: String,
    http: reqwest::Client,
}

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
    pub slug: String,
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
    pub slug: String,
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
    pub documents: Vec<Document>,
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
    pub title: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentUpdate {
    pub expected_hash: String,
    pub documents: Documents,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityUpdate {
    pub expected_hash: String,
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
    let client = Client::load()?;
    read_summary_page(&client, filters).await
}

#[derive(Deserialize)]
struct ArticleResponse<T> {
    article: T,
}

impl Client {
    fn load() -> Result<Self, ApiError> {
        let api_key = match vesper_credentials::consumer_api(ConsumerApi::Knowledge)? {
            Stored::Ready(api_key) => api_key,
            Stored::Missing => return Err(ApiError::MissingCredentials("my-knowledge")),
        };
        Ok(Self {
            api_key,
            http: reqwest::Client::builder()
                .timeout(super::REQUEST_TIMEOUT)
                .build()?,
        })
    }
}

pub async fn list(cursor: Option<String>) -> Result<Page, ApiError> {
    let client = Client::load()?;
    let page = read_summary_page(
        &client,
        &ListFilters {
            cursor,
            limit: Some(20),
            ..ListFilters::default()
        },
    )
    .await?;
    let documents = read_documents(&client, page.articles).await?;
    Ok(Page {
        documents,
        cursor: page.cursor,
    })
}

pub async fn overview() -> Result<Page, ApiError> {
    let client = Client::load()?;
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
    let documents = read_documents(&client, overview_summaries(summaries)).await?;
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
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "list knowledge articles",
            status,
        });
    }
    Ok(response.json().await?)
}

async fn read_documents(
    client: &Client,
    summaries: Vec<Summary>,
) -> Result<Vec<Document>, ApiError> {
    stream::iter(summaries)
        .map(|summary| {
            let client = client.clone();
            async move {
                let response = client
                    .http
                    .get(format!("{ENDPOINT}/{}", summary.id))
                    .bearer_auth(&client.api_key)
                    .send()
                    .await?;
                let status = response.status();
                if !status.is_success() {
                    return Err(ApiError::Status {
                        operation: "read knowledge article",
                        status,
                    });
                }
                let result: ArticleResponse<Article> = response.json().await?;
                let article = result.article;
                project_article(article).await
            }
        })
        .buffered(READ_CONCURRENCY)
        .try_collect()
        .await
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
        if latest.is_none_or(|current| summary.created_at > current.created_at) {
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
    let compiled = match compile_knowledge_enriched(&source).await {
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
        slug: article.slug,
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

pub fn latest_newspaper_issues(documents: &[Document]) -> NewspaperIssues {
    let mut developer = None;
    let mut personal = None;
    for document in documents {
        let issue = match document.newspaper_edition {
            Some(NewspaperEdition::Developer) => &mut developer,
            Some(NewspaperEdition::Personal) => &mut personal,
            None => continue,
        };
        if issue.is_none_or(|current| is_newer(document, current)) {
            *issue = Some(document);
        }
    }
    NewspaperIssues {
        developer: developer.map(|document| document.id.clone()),
        personal: personal.map(|document| document.id.clone()),
    }
}

fn is_newer(candidate: &Document, current: &Document) -> bool {
    match (
        OffsetDateTime::parse(&candidate.created_at, &Rfc3339),
        OffsetDateTime::parse(&current.created_at, &Rfc3339),
    ) {
        (Ok(candidate), Ok(current)) => candidate > current,
        _ => candidate.created_at > current.created_at,
    }
}

pub async fn get(id: &str) -> Result<Article, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .get(format!("{ENDPOINT}/{id}"))
        .bearer_auth(&client.api_key)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "read knowledge article",
            status,
        });
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn create(input: &Create) -> Result<Article, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .post(ENDPOINT)
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::CREATED {
        return Err(ApiError::Status {
            operation: "create knowledge article",
            status,
        });
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn update_draft(id: &str, input: &DraftUpdate) -> Result<Article, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .patch(format!("{ENDPOINT}/{id}"))
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "update knowledge article",
            status,
        });
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn update_documents(id: &str, input: &DocumentUpdate) -> Result<Article, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .patch(format!("{ENDPOINT}/{id}"))
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "update knowledge documents",
            status,
        });
    }
    let result: ArticleResponse<Article> = response.json().await?;
    Ok(result.article)
}

pub async fn set_visibility(id: &str, input: &VisibilityUpdate) -> Result<Summary, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .patch(format!("{ENDPOINT}/{id}"))
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "set knowledge visibility",
            status,
        });
    }
    let result: ArticleResponse<Summary> = response.json().await?;
    Ok(result.article)
}

pub async fn delete(id: &str, expected_hash: &str) -> Result<(), ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .delete(format!("{ENDPOINT}/{id}"))
        .bearer_auth(&client.api_key)
        .json(&serde_json::json!({ "expectedHash": expected_hash }))
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn projected_document(id: &str, tags: &[&str], created_at: &str) -> Document {
        project_article(Article {
            id: id.to_owned(),
            slug: id.to_owned(),
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

    #[tokio::test]
    async fn overview_follows_pages_and_rejects_cursor_cycles() {
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
                    slug: id.to_owned(),
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
    fn decodes_list_metadata() {
        let page: ArticlePage = serde_json::from_value(serde_json::json!({
            "articles": [{
                "id": "019c1234-1234-7000-8000-123456789abc",
                "slug": "typed-boundaries",
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
    }

    #[test]
    fn overview_keeps_regular_articles_and_latest_newspapers() {
        fn summary(id: &str, tags: &[&str], created_at: &str) -> Summary {
            Summary {
                id: id.to_owned(),
                slug: id.to_owned(),
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
    fn decodes_markdown() {
        let response: ArticleResponse<Article> = serde_json::from_value(serde_json::json!({
            "article": {
                "id": "019c1234-1234-7000-8000-123456789abc",
                "slug": "typed-boundaries",
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
            "slug": "daily-brief",
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

        assert_eq!(document.source, "## Today\n\nNews");
        assert!(document.html.starts_with("<h2 id=\"today\">Today</h2>"));
        assert_eq!(document.tags, ["newspaper", "daily"]);
    }

    #[tokio::test]
    async fn preserves_article_when_embed_enrichment_fails() {
        let article: Article = serde_json::from_value(serde_json::json!({
            "id": "019c1234-1234-7000-8000-123456789abc",
            "slug": "unavailable-embed",
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
        let documents = vec![
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
            latest_newspaper_issues(&documents),
            NewspaperIssues {
                developer: Some("developer".to_owned()),
                personal: Some("personal".to_owned()),
            }
        );
    }
}
