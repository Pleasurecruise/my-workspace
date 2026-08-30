use super::*;

fn test_store() -> (PathBuf, Store) {
    let directory = std::env::temp_dir().join(format!("vesper-todo-{}", uuid::Uuid::new_v4()));
    (directory.clone(), Store::new(directory.join(FILE_NAME)))
}

#[tokio::test]
async fn handles_crud() {
    let (directory, store) = test_store();
    let date = "2026-08-23";
    let created = store.create(date, "  Ship CLI  ").await.unwrap();
    let id = created.items[0].id.clone();
    assert_eq!(created.items[0].text, "Ship CLI");
    assert_eq!(store.get(date, &id).await.unwrap().id, id);
    assert_eq!(
        store
            .update(date, &id, "Ship Todo CLI")
            .await
            .unwrap()
            .items[0]
            .text,
        "Ship Todo CLI"
    );
    assert!(store.set_completed(date, &id, true).await.unwrap().items[0].completed);
    assert!(store.delete(date, &id).await.unwrap().items.is_empty());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn isolates_dates() {
    let (directory, store) = test_store();
    store.create("2026-08-22", "Yesterday").await.unwrap();
    store.create("2026-08-23", "Today").await.unwrap();
    assert_eq!(
        store.list("2026-08-22").await.unwrap().items[0].text,
        "Yesterday"
    );
    assert_eq!(
        store.list("2026-08-23").await.unwrap().items[0].text,
        "Today"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn reloads_before_mutation() {
    let (directory, first) = test_store();
    let second = Store::new(directory.join(FILE_NAME));
    first.create("2026-08-23", "First").await.unwrap();
    second.create("2026-08-23", "Second").await.unwrap();
    assert_eq!(first.list("2026-08-23").await.unwrap().items.len(), 2);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn serializes_writers() {
    let (directory, first) = test_store();
    let second = Store::new(directory.join(FILE_NAME));
    let (first_result, second_result) = tokio::join!(
        first.create("2026-08-23", "First"),
        second.create("2026-08-23", "Second")
    );
    first_result.unwrap();
    second_result.unwrap();
    assert_eq!(first.list("2026-08-23").await.unwrap().items.len(), 2);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn ignores_legacy_storage() {
    let (directory, store) = test_store();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("today-todos.json"),
        r#"{"date":"2026-08-23","items":[{"id":"legacy","text":"Keep me","completed":false}]}"#,
    )
    .unwrap();

    assert!(store.list("2026-08-23").await.unwrap().items.is_empty());
    assert!(!directory.join(FILE_NAME).exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn rejects_long_text() {
    let (directory, store) = test_store();
    let error = store
        .create("2026-08-23", &"x".repeat(MAX_TEXT_LENGTH + 1))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::TextTooLong));
    if directory.exists() {
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn computes_midnight_delay() {
    let now = time::macros::datetime!(2026-08-24 23:59:30 +8);

    assert_eq!(
        rollover_delay_at(now, time::macros::offset!(+8)).unwrap(),
        std::time::Duration::from_secs(30)
    );
}
