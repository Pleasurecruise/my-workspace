use super::*;

#[tokio::test]
async fn dated_check_ins() {
    let directory = tempfile::tempdir().unwrap();
    let store = todo_core::Store::new(directory.path().join("vesper.sqlite3"));
    let ids = vec!["read".to_owned()];
    run_with_store(&store, "2024-02-29", "check-in", &ids)
        .await
        .unwrap();
    run_with_store(&store, "2024-02-29", "check-ins", &ids)
        .await
        .unwrap();
    assert!(
        store
            .read_check_ins(ids.clone(), "2024-02-29")
            .await
            .unwrap()[0]
            .completed
    );
    assert!(
        !store
            .read_check_ins(ids.clone(), "2024-03-01")
            .await
            .unwrap()[0]
            .completed
    );
    run_with_store(&store, "2024-02-29", "undo-check-in", &ids)
        .await
        .unwrap();
    assert!(
        !store
            .read_check_ins(ids.clone(), "2024-02-29")
            .await
            .unwrap()[0]
            .completed
    );
    assert!(
        run_with_store(&store, "9999-12-31", "check-in", &ids)
            .await
            .is_err()
    );
    assert!(
        run_with_store(&store, "2024-02-29", "check-ins", &[])
            .await
            .is_err()
    );
    assert!(!store.schedule_directory().exists());
}

#[tokio::test]
async fn rejects_bad_todo_args() {
    let directory = std::env::temp_dir().join(format!("vesper-cli-todo-{}", uuid::Uuid::new_v4()));
    let store = todo_core::Store::new(directory.join("vesper.sqlite3"));

    let error = run_with_store(&store, "2026-08-23", "create", &[])
        .await
        .unwrap_err();

    assert!(error.contains("invalid todo arguments"));
    assert!(!directory.exists());
}
