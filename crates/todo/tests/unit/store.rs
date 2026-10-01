use super::*;

#[test]
fn shared_paths() {
    let store = Store::shared().unwrap();
    assert_eq!(store.database_path(), database::path().unwrap());
    assert_eq!(
        store.schedule_directory(),
        dirs::data_dir().unwrap().join("me.you-find.vesper/ics"),
    );
}

#[tokio::test]
async fn lists_while_writing() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(database::FILE_NAME));
    store.create("2026-09-07", "Committed", None).await.unwrap();
    let mut writer = database::open(store.database_path()).unwrap();
    writer
        .batch_execute("BEGIN IMMEDIATE; UPDATE todo_items SET text = 'Pending';")
        .unwrap();
    let result = store.list("2026-09-07").await;
    writer.batch_execute("ROLLBACK;").unwrap();
    assert_eq!(result.unwrap().items[0].text, "Committed");
}

pub(super) fn test_store() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(database::FILE_NAME));
    (directory, store)
}

#[tokio::test]
async fn handles_crud() {
    let (_directory, store) = test_store();
    let date = "2026-08-23";
    let created = store.create(date, "  Ship CLI  ", None).await.unwrap();
    let id = created.items[0].id.clone();
    assert_eq!(created.items[0].text, "Ship CLI");
    assert_eq!(store.get(date, &id).await.unwrap().id, id);
    assert_eq!(
        store
            .update(date, &id, "Ship Todo CLI", None)
            .await
            .unwrap()
            .items[0]
            .text,
        "Ship Todo CLI"
    );
    assert!(store.set_completed(date, &id, true).await.unwrap().items[0].completed);
    store.set_completed(date, &id, false).await.unwrap();
    assert!(store.delete(date, &id).await.unwrap().items.is_empty());
}

#[tokio::test]
async fn isolates_dates() {
    let (_directory, store) = test_store();
    store.create("2026-08-22", "Yesterday", None).await.unwrap();
    store.create("2026-08-23", "Today", None).await.unwrap();
    assert_eq!(
        store.list("2026-08-22").await.unwrap().items[0].text,
        "Yesterday"
    );
    assert_eq!(
        store.list("2026-08-23").await.unwrap().items[0].text,
        "Today"
    );
}

#[tokio::test]
async fn reloads_before_mutation() {
    let (directory, first) = test_store();
    let second = Store::new(directory.path().join(database::FILE_NAME));
    first.create("2026-08-23", "First", None).await.unwrap();
    second.create("2026-08-23", "Second", None).await.unwrap();
    assert_eq!(first.list("2026-08-23").await.unwrap().items.len(), 2);
}

#[tokio::test]
async fn rolls_back_failed_mutation() {
    use diesel::connection::SimpleConnection;
    let (_directory, store) = test_store();
    let original = store.create("2026-08-23", "Keep me", None).await.unwrap();
    let mut connection = database::open(store.database_path()).unwrap();
    connection.batch_execute("CREATE TRIGGER reject_todo BEFORE INSERT ON todo_items BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(store.create("2026-08-23", "Fail", None).await.is_err());
    assert_eq!(store.list("2026-08-23").await.unwrap(), original);
    drop(connection);
}

#[tokio::test]
async fn serializes_writers() {
    let (directory, first) = test_store();
    let second = Store::new(directory.path().join(database::FILE_NAME));
    let (first_result, second_result) = tokio::join!(
        first.create("2026-08-23", "First", None),
        second.create("2026-08-23", "Second", None)
    );
    first_result.unwrap();
    second_result.unwrap();
    assert_eq!(first.list("2026-08-23").await.unwrap().items.len(), 2);
}

#[tokio::test]
async fn rejects_long_text() {
    let (_directory, store) = test_store();
    let error = store
        .create("2026-08-23", &"x".repeat(MAX_TEXT_LENGTH + 1), None)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::TextTooLong));
}

#[tokio::test]
async fn edits_preserve_task_state() {
    let (_directory, store) = test_store();
    let date = "2026-09-10";
    let list = store
        .create(date, "  Read  ", Some("  Chapter one\nTake notes  "))
        .await
        .unwrap();
    let id = &list.items[0].id;
    assert_eq!(
        list.items[0].description.as_deref(),
        Some("Chapter one\nTake notes")
    );
    store.set_rollover(date, id, true).await.unwrap();
    let list = store
        .update(date, id, "Read more", Some("Chapter two"))
        .await
        .unwrap();
    assert!(list.items[0].rollover);
    assert!(!list.items[0].completed);
    assert_eq!(list.date, date);
    assert!(list.items[0].details.is_none());
    store.update(date, id, "Title only", None).await.unwrap();
    assert_eq!(
        store.get(date, id).await.unwrap().description.as_deref(),
        Some("Chapter two")
    );
    assert!(matches!(
        store.update(date, id, "Bad", Some(&"x".repeat(4001))).await,
        Err(Error::DescriptionTooLong)
    ));
    assert_eq!(store.get(date, id).await.unwrap().text, "Title only");
    let reopened = Store::new(store.database_path().to_owned());
    assert_eq!(
        reopened.get(date, id).await.unwrap().description.as_deref(),
        Some("Chapter two")
    );
    reopened
        .update(date, id, "Clear", Some("  "))
        .await
        .unwrap();
    assert!(reopened.get(date, id).await.unwrap().description.is_none());
}

#[tokio::test]
async fn validates_saved_order() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    let store = Store::new(path.clone());
    let date = "2026-09-12";
    store.create(date, "First", None).await.unwrap();
    let original = store.create(date, "Second", None).await.unwrap();
    let ids: Vec<String> = original
        .items
        .iter()
        .rev()
        .map(|item| item.id.clone())
        .collect();
    let reordered = store.reorder(date, ids.clone()).await.unwrap();
    assert_eq!(reordered.items[0].text, "Second");
    assert_eq!(Store::new(path).list(date).await.unwrap(), reordered);
    for invalid in [
        vec![ids[0].clone()],
        vec![ids[0].clone(), ids[0].clone()],
        vec![ids[0].clone(), "missing".into()],
    ] {
        assert!(matches!(
            store.reorder(date, invalid).await,
            Err(Error::InvalidOrder)
        ));
        assert_eq!(store.list(date).await.unwrap(), reordered);
    }
    assert!(matches!(
        store.reorder("2026-09-13", ids.clone()).await,
        Err(Error::InvalidOrder)
    ));
    store
        .create(date, "New in another window", None)
        .await
        .unwrap();
    assert!(matches!(
        store.reorder(date, ids).await,
        Err(Error::InvalidOrder)
    ));
    assert_eq!(store.list(date).await.unwrap().items.len(), 3);
}

#[tokio::test]
async fn rollover_respects_opt_out() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(database::FILE_NAME));
    let first = "2026-09-12";
    let ids = store
        .create(first, "Carry", Some("Keep notes"))
        .await
        .unwrap()
        .items;
    let id = ids[0].id.clone();
    assert!(!ids[0].rollover);
    store.create(first, "Leave here", None).await.unwrap();
    let complete = store.create(first, "Done", None).await.unwrap().items[2]
        .id
        .clone();
    store.set_rollover(first, &complete, true).await.unwrap();
    store.set_completed(first, &complete, true).await.unwrap();
    store.set_rollover(first, &id, true).await.unwrap();
    store
        .create("2026-09-15", "Existing today", None)
        .await
        .unwrap();
    assert!(store.roll_over(first).await.unwrap().is_empty());
    assert!(store.list("2026-09-16").await.unwrap().items.is_empty());
    assert_eq!(
        store.roll_over("2026-09-15").await.unwrap(),
        vec![first, "2026-09-15"]
    );
    assert_eq!(store.list(first).await.unwrap().items.len(), 2);
    let moved = store.get("2026-09-15", &id).await.unwrap();
    assert!(moved.rollover);
    assert_eq!(moved.description.as_deref(), Some("Keep notes"));
    assert_eq!(
        store.list("2026-09-15").await.unwrap().items[0].text,
        "Existing today"
    );
    assert!(store.roll_over("2026-09-15").await.unwrap().is_empty());
    store.roll_over("2026-09-16").await.unwrap();
    store.set_rollover("2026-09-16", &id, false).await.unwrap();
    assert!(store.roll_over("2026-09-17").await.unwrap().is_empty());
    store.set_rollover("2026-09-16", &id, true).await.unwrap();
    store.set_completed("2026-09-16", &id, true).await.unwrap();
    assert!(store.roll_over("2026-09-17").await.unwrap().is_empty());
}

#[tokio::test]
async fn rollover_is_atomic() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    let store = Store::new(path.clone());
    let other = Store::new(path.clone());
    let id = store
        .create("2026-09-12", "Carry", None)
        .await
        .unwrap()
        .items[0]
        .id
        .clone();
    store.set_rollover("2026-09-12", &id, true).await.unwrap();
    let mut connection = database::open(&path).unwrap();
    connection.batch_execute("CREATE TRIGGER fail_rollover BEFORE INSERT ON todo_items WHEN NEW.date = '2026-09-13' BEGIN SELECT RAISE(FAIL, 'write failed'); END;").unwrap();
    assert!(store.roll_over("2026-09-13").await.is_err());
    assert_eq!(store.list("2026-09-12").await.unwrap().items.len(), 1);
    assert!(store.list("2026-09-13").await.unwrap().items.is_empty());
    connection
        .batch_execute("DROP TRIGGER fail_rollover")
        .unwrap();
    let (first, second) =
        tokio::join!(store.roll_over("2026-09-13"), other.roll_over("2026-09-13"));
    assert_eq!(first.unwrap().len() + second.unwrap().len(), 2);
    assert_eq!(
        Store::new(path).list("2026-09-13").await.unwrap().items[0].id,
        id
    );
}

#[tokio::test]
async fn derives_completed_days() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(database::FILE_NAME));
    let date = "2024-02-29";
    let ids = vec!["read".to_owned(), "walk".to_owned()];
    assert_eq!(store.read_days(vec![], date).await.unwrap().len(), 29);
    let list = store.create(date, "Finish report", None).await.unwrap();
    let id = &list.items[0].id;
    store.set_check_in("read", date, true).await.unwrap();
    store.set_check_in("walk", date, true).await.unwrap();
    assert!(store.read_days(ids.clone(), date).await.unwrap().is_empty());
    store.set_completed(date, id, true).await.unwrap();
    assert_eq!(
        store.read_days(ids.clone(), "2024-02-01").await.unwrap(),
        vec![date]
    );
    store.set_check_in("walk", date, false).await.unwrap();
    assert!(store.read_days(ids.clone(), date).await.unwrap().is_empty());
    assert_eq!(
        store
            .read_days(vec!["read".into(), "read".into()], date)
            .await
            .unwrap(),
        vec![date]
    );
    store.set_completed(date, id, false).await.unwrap();
    store.delete(date, id).await.unwrap();
    assert_eq!(
        store.read_days(vec!["read".into()], date).await.unwrap(),
        vec![date]
    );
    assert_eq!(store.read_days(vec![], date).await.unwrap().len(), 29);
    assert!(store.read_days(ids, "2024-03-01").await.unwrap().is_empty());
}

#[tokio::test]
async fn completed_days_exclude_future() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(database::FILE_NAME));
    let list = store
        .create("9999-12-31", "Future task", None)
        .await
        .unwrap();
    store
        .set_completed("9999-12-31", &list.items[0].id, true)
        .await
        .unwrap();
    assert!(
        store
            .read_days(vec![], "9999-12-31")
            .await
            .unwrap()
            .is_empty()
    );
    let list = store.create("2024-02-01", "Done", None).await.unwrap();
    store
        .set_completed("2024-02-01", &list.items[0].id, true)
        .await
        .unwrap();
    assert_eq!(
        store.read_days(vec![], "2024-02-29").await.unwrap(),
        (1..=29)
            .map(|day| format!("2024-02-{day:02}"))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn completes_empty_days() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(database::FILE_NAME));
    let today = crate::current_date().unwrap();
    let days = store.read_days(vec![], &today).await.unwrap();
    assert_eq!(days.last(), Some(&today));
    assert_eq!(days.len(), usize::from(parse_date(&today).unwrap().day()));
    let list = store.create(&today, "Pending", None).await.unwrap();
    assert!(
        !store
            .read_days(vec![], &today)
            .await
            .unwrap()
            .contains(&today)
    );
    store.delete(&today, &list.items[0].id).await.unwrap();
    assert!(
        store
            .read_days(vec![], &today)
            .await
            .unwrap()
            .contains(&today)
    );
    assert!(
        store
            .read_days(vec!["walk".into()], &today)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn completed_tasks_require_reopening() {
    let (directory, store) = test_store();
    let date = "2026-09-30";
    let list = store
        .create(date, "Original", Some("Keep notes"))
        .await
        .unwrap();
    let id = &list.items[0].id;
    store.set_completed(date, id, true).await.unwrap();
    // A second client must check the current stored state, not its stale open projection.
    let stale = Store::new(directory.path().join(database::FILE_NAME));
    assert!(matches!(
        stale.update(date, id, "Changed", Some("Lost notes")).await,
        Err(Error::CompletedItem)
    ));
    assert!(matches!(
        stale.delete(date, id).await,
        Err(Error::CompletedItem)
    ));
    assert!(matches!(
        stale.set_rollover(date, id, true).await,
        Err(Error::CompletedItem)
    ));
    let completed = stale.get(date, id).await.unwrap();
    let wire = serde_json::to_value(&completed).unwrap();
    assert_eq!(wire["sourceOwned"], false);
    assert_eq!(wire["completed"], true);
    assert_eq!(completed.text, "Original");
    assert_eq!(completed.description.as_deref(), Some("Keep notes"));
    assert!(!completed.rollover);
    stale.set_completed(date, id, false).await.unwrap();
    stale.update(date, id, "Changed", None).await.unwrap();
    stale.set_rollover(date, id, true).await.unwrap();
    stale.delete(date, id).await.unwrap();
    assert!(stale.list(date).await.unwrap().items.is_empty());
}
