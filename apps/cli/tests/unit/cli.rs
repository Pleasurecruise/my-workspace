use super::run;

#[tokio::test]
async fn reads_content_files_without_changing_newlines() {
    let path = std::env::temp_dir().join(format!("vesper-cli-input-{}.md", uuid::Uuid::new_v4()));
    let content = "# 标题\n\n第一段\n第二段\n";
    tokio::fs::write(&path, content).await.unwrap();
    let input = super::read_input(&["--file".to_owned(), path.display().to_string()]).await;
    tokio::fs::remove_file(&path).await.unwrap();
    assert_eq!(input.unwrap(), content);
    assert!(
        super::read_input(&["--file".to_owned(), path.display().to_string()])
            .await
            .is_err()
    );
    assert!(super::read_input(&["--file".to_owned()]).await.is_err());
    assert!(
        super::read_input(&["--stdin".to_owned(), "extra".to_owned()])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn rejects_invalid_consumer_queries_before_authentication() {
    for arguments in [
        vec!["memo", "get", ""],
        vec!["moment", "get", ""],
        vec!["moment", "query", r#"{"search":"rust","tags":["code"]}"#],
        vec!["moment", "query", r#"{"fromDate":"2026-02-30"}"#],
        vec![
            "moment",
            "query",
            r#"{"fromDate":"2026-09-05","toDate":"2026-09-01"}"#,
        ],
        vec!["moment", "query", r#"{"limit":101}"#],
        vec!["knowledge", "page", r#"{"limit":0}"#],
        vec!["knowledge", "page", r#"{"tags":[""]}"#],
        vec!["knowledge", "page", r#"{"unknown":true}"#],
        vec!["status", "unknown"],
        vec!["status", "codex", "claude"],
    ] {
        let result = run(arguments.into_iter().map(str::to_owned)).await;
        let error = result.expect_err("invalid arguments must fail");
        assert!(
            !error.contains("configured"),
            "validation should precede credentials: {error}"
        );
    }
}

#[tokio::test]
async fn rejects_unknown_args() {
    let error = run(["unknown".to_owned()].into_iter()).await.unwrap_err();
    assert!(error.contains("invalid arguments"));
}

#[tokio::test]
async fn rejects_bad_todo_date() {
    let error = run([
        "todo".to_owned(),
        "--date".to_owned(),
        "not-a-date".to_owned(),
        "list".to_owned(),
    ]
    .into_iter())
    .await
    .unwrap_err();

    assert_eq!(error, "invalid Todo date not-a-date; expected YYYY-MM-DD");
}
