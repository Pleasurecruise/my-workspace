use super::ApiError;
use futures_util::stream::{self, StreamExt, TryStreamExt};
use md_dialect::{TocEntry, compile_knowledge_enriched, compile_knowledge_plain, knowledge_body};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use vesper_credentials::{ConsumerApi, Stored};

const ENDPOINT: &str = "https://knowledge.you-find.me/api/articles";
const READ_CONCURRENCY: usize = 6;

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

#[derive(Deserialize)]
struct ArticlePage {
    articles: Vec<Summary>,
    cursor: Option<String>,
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
    let mut request = client
        .http
        .get(ENDPOINT)
        .bearer_auth(&client.api_key)
        .query(&[("limit", "20")]);
    if let Some(cursor) = cursor.as_deref() {
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
    let page: ArticlePage = response.json().await?;
    let documents = stream::iter(page.articles)
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
        .await?;
    Ok(Page {
        documents,
        cursor: page.cursor,
    })
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
