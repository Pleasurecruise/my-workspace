use super::{
    ApiError, Article, ArticlePage, Client, Document, EditionSummary, ListFilters,
    OVERVIEW_PAGE_SIZE, Summary, article_identity, read_summary_page,
};
use crate::api::credentials::ConsumerApi;
use markdown::{ArticleMetadata, article_ids, article_urls, knowledge};
use std::collections::{HashMap, HashSet};

pub async fn project_article(article: Article) -> Result<Document, ApiError> {
    let edition = article.editions.get("zh").ok_or_else(|| {
        ApiError::Protocol(format!("article {} has no Chinese edition", article.id))
    })?;
    let source = knowledge::body(&edition.markdown).to_owned();
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
    let compiled = match knowledge::compile(&source, metadata).await {
        Ok(compiled) => compiled,
        Err(error) => {
            tracing::warn!(
                article_id = %article.id,
                %error,
                "could not enrich Knowledge embeds; preserving them as code blocks"
            );
            knowledge::fallback(&source)
        }
    };
    Ok(Document {
        id: article.id,
        title: edition.title.clone(),
        summary: edition.summary.clone(),
        tags: article.tags,
        visibility: article.visibility,
        content_hash: article.content_hash,
        created_at: article.created_at,
        updated_at: article.updated_at,
        source,
        html: compiled.html,
        toc: compiled.toc,
        stats: compiled.stats,
    })
}

/// Compile an editor block with document definitions and authorized article references.
pub async fn preview(source: &str, context: &str) -> Result<knowledge::Compiled, String> {
    let source = markdown::fragment(source, context);
    let ids = article_ids(&source).map_err(|error| error.to_string())?;
    let urls: Vec<_> = article_urls(&source)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|url| article_identity(url).is_some())
        .collect();
    let mut metadata = HashMap::new();
    if !ids.is_empty() || !urls.is_empty() {
        for summary in reference_summaries()
            .await
            .map_err(|error| error.to_string())?
        {
            resolve_card_metadata(&summary, &ids, &urls, &mut metadata);
        }
    }
    knowledge::compile(&source, metadata)
        .await
        .map_err(|error| error.to_string())
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

#[cfg(test)]
#[path = "../../../tests/unit/knowledge_render.rs"]
mod tests;
