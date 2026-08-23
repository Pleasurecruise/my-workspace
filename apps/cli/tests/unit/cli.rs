use super::run;

#[tokio::test]
async fn rejects_unknown_arguments() {
    let error = run(["unknown".to_owned()].into_iter()).await.unwrap_err();
    assert!(error.contains("invalid arguments"));
}
