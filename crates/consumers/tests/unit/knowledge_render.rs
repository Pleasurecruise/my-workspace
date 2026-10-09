use super::super::{Edition, Visibility, project_summary};
use super::*;

#[tokio::test]
async fn compiles_draft_dialects_and_reports_invalid_source() {
    let source = "```embed:annotation\nmark: Example\nnote: Kept note\n---\nExample body\n```\n\n:suzume_思考:";
    let result = preview(source, source).await.unwrap();
    assert!(result.html.contains("content-embed-annotation"));
    assert!(result.html.contains("Kept note"));
    assert!(result.html.contains("markdown-emoji"));
    assert!(
        preview("```embed:annotation\nnote: Incomplete\n```", "")
            .await
            .is_err()
    );
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
            next_cursor: if daily && filters.cursor.is_none() {
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
            next_cursor: Some("loop".into())
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
        "nextCursor": "next-page"
    }))
    .expect("valid my-knowledge list response");

    assert_eq!(page.next_cursor.as_deref(), Some("next-page"));
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
