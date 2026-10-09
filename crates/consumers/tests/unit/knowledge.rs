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
    let source = include_str!("../fixtures/knowledge-contract.json");
    let responses: Responses = serde_json::from_str(source).unwrap();
    assert!(responses.list.next_cursor.is_none());
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
    let search = project_summary(responses.search.articles.into_iter().next().unwrap()).unwrap();
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

#[test]
fn decodes_article_ids() {
    for (path, id) in [
        ("%61lpha", "alpha"),
        ("%e4%b8%ad%e6%96%87", "中文"),
        ("a%23b", "a#b"),
        ("a%2520b", "a%20b"),
    ] {
        assert_eq!(
            article_identity(&format!("https://knowledge.you-find.me/articles/{path}")).as_deref(),
            Some(id)
        );
    }
    assert!(article_identity("https://knowledge.you-find.me/articles/%FF").is_none());
    assert!(article_identity("https://example.com/articles/alpha").is_none());
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
            next_cursor: next.map(str::to_owned),
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
            next_cursor: Some("same".to_owned()),
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
            next_cursor: Some("next".to_owned()),
        })
    })
    .await
    .expect_err("failed page must not become a complete overview");
    assert!(error.to_string().contains("second page failed"));
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

#[test]
fn index_entries_omit_document_bodies() {
    let summary = Summary {
        id: "article-1".to_owned(),
        editions: HashMap::from([(
            "zh".to_owned(),
            EditionSummary {
                title: "Article".to_owned(),
                summary: "Summary".to_owned(),
            },
        )]),
        tags: Vec::new(),
        visibility: Visibility::Private,
        content_hash: "hash".to_owned(),
        created_at: "2026-09-12T10:00:00Z".to_owned(),
        updated_at: "2026-09-12T10:00:00Z".to_owned(),
    };
    let entry = serde_json::to_value(project_summary(summary).unwrap()).unwrap();
    assert_eq!(entry["id"], "article-1");
    for field in ["source", "html", "toc"] {
        assert!(entry.get(field).is_none(), "{field}");
    }
}
