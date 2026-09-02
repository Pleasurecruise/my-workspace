use super::*;

#[tokio::test]
async fn rejects_bad_todo_args() {
    let directory = std::env::temp_dir().join(format!("vesper-cli-todo-{}", uuid::Uuid::new_v4()));
    let store = todo_core::Store::new(directory.join("todos.json"));

    let error = run_with_store(&store, "2026-08-23", "create", &[])
        .await
        .unwrap_err();

    assert!(error.contains("invalid todo arguments"));
    assert!(!directory.exists());
}
