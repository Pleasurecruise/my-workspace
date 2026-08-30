use super::run;

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
