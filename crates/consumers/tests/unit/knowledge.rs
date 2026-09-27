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
