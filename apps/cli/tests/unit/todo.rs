use super::*;

#[tokio::test]
async fn dated_check_ins() {
    let directory = tempfile::tempdir().unwrap();
    let store = ::todo::Store::new(directory.path().join("vesper.sqlite3"));
    let ids = vec!["read".to_owned()];
    dispatch(&store, "2024-02-29", "check-in", &ids)
        .await
        .unwrap();
    dispatch(&store, "2024-02-29", "check-ins", &ids)
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
    dispatch(&store, "2024-02-29", "undo-check-in", &ids)
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
        dispatch(&store, "9999-12-31", "check-in", &ids)
            .await
            .is_err()
    );
    assert!(
        dispatch(&store, "2024-02-29", "check-ins", &[])
            .await
            .is_err()
    );
    assert!(!store.schedule_directory().exists());
}

#[tokio::test]
async fn rejects_bad_todo_args() {
    let directory = std::env::temp_dir().join(format!("vesper-cli-todo-{}", uuid::Uuid::new_v4()));
    let store = ::todo::Store::new(directory.join("vesper.sqlite3"));

    let error = dispatch(&store, "2026-08-23", "create", &[])
        .await
        .unwrap_err();

    assert!(error.contains("invalid todo arguments"));
    assert!(!directory.exists());
}

#[tokio::test]
async fn calendar_rollover() {
    let directory = tempfile::tempdir().unwrap();
    let store = ::todo::Store::new(directory.path().join("vesper.sqlite3"));
    let today = ::todo::current_date().unwrap();
    let previous = "2024-02-29";
    let list = store.create(previous, "Carry", None).await.unwrap();
    let id = &list.items[0].id;
    store.set_rollover(previous, id, true).await.unwrap();
    dispatch(&store, "9999-12-31", "sync-ics", &[])
        .await
        .unwrap();
    assert!(store.get(&today, id).await.is_ok());
    assert!(store.get("9999-12-31", id).await.is_err());
}

#[tokio::test]
async fn invalid_import() {
    let directory = tempfile::tempdir().unwrap();
    let store = ::todo::Store::new(directory.path().join("vesper.sqlite3"));
    let previous = "2024-02-29";
    let list = store
        .create(previous, "Keep on original day", None)
        .await
        .unwrap();
    let id = &list.items[0].id;
    store.set_rollover(previous, id, true).await.unwrap();
    let missing = directory
        .path()
        .join("missing.ics")
        .to_string_lossy()
        .into_owned();
    assert!(
        dispatch(&store, previous, "import-ics", &[missing])
            .await
            .is_err()
    );
    assert!(store.get(previous, id).await.is_ok());
}
