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
async fn recovers_a_completed_temporary_write() {
    let (directory, store) = test_store();
    store.create("2026-08-23", "Recover me").await.unwrap();
    let path = directory.join(FILE_NAME);
    std::fs::rename(&path, path.with_extension("json.tmp")).unwrap();

    let recovered = Store::new(path).list("2026-08-23").await.unwrap();
    assert_eq!(recovered.items[0].text, "Recover me");
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
async fn ignores_old_file() {
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

#[tokio::test]
async fn imports_once() {
    let (directory, store) = test_store();
    std::fs::create_dir_all(store.schedule_directory()).unwrap();
    std::fs::write(
        store.schedule_directory().join("work.ics"),
        "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\nDTSTART:20260823T093000\nRRULE:FREQ=DAILY\nEND:VEVENT\nEND:VCALENDAR\n",
    )
    .unwrap();

    let first = store.sync_schedule("2026-08-23").await.unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].text, "09:30 Standup");
    assert_eq!(
        first.items[0]
            .details
            .as_ref()
            .map(|details| details.calendar.as_str()),
        Some("work.ics")
    );
    let id = first.items[0].id.clone();
    assert_eq!(
        store.sync_schedule("2026-08-23").await.unwrap().items.len(),
        1
    );
    store.delete("2026-08-23", &id).await.unwrap();
    assert!(
        store
            .sync_schedule("2026-08-23")
            .await
            .unwrap()
            .items
            .is_empty()
    );
    store.create("2026-08-23", "09:30 Standup").await.unwrap();
    let synced = store.sync_schedule("2026-08-23").await.unwrap();
    assert_eq!(synced.items.len(), 1);
    assert!(synced.items[0].details.is_none());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn combines_calendars() {
    let (directory, store) = test_store();
    let sources = directory.join("sources");
    std::fs::create_dir_all(&sources).unwrap();
    let mut paths = Vec::new();
    for (name, summary) in [("work.ics", "Standup"), ("personal.ics", "Exercise")] {
        let path = sources.join(name);
        std::fs::write(
            &path,
            format!(
                "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:daily\nSUMMARY:{summary}\nDTSTART:20260823\nEND:VEVENT\nEND:VCALENDAR\n"
            ),
        )
        .unwrap();
        paths.push(path);
    }

    let installed = store.import_schedules(&paths).await.unwrap();
    assert_eq!(installed.len(), 2);
    let todos = store.sync_schedule("2026-08-23").await.unwrap();
    assert_eq!(todos.items.len(), 2);
    assert_eq!(
        todos
            .items
            .iter()
            .map(|item| item.text.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["Exercise", "Standup"])
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn rejects_schedule_names_that_collide_case_insensitively() {
    let (directory, store) = test_store();
    let first_directory = directory.join("first");
    let second_directory = directory.join("second");
    std::fs::create_dir_all(&first_directory).unwrap();
    std::fs::create_dir_all(&second_directory).unwrap();
    let calendar = "BEGIN:VCALENDAR\nEND:VCALENDAR\n";
    let first = first_directory.join("Work.ics");
    let second = second_directory.join("work.ics");
    std::fs::write(&first, calendar).unwrap();
    std::fs::write(&second, calendar).unwrap();

    assert!(matches!(
        store.import_schedules(&[first, second]).await,
        Err(Error::DuplicateScheduleName(_))
    ));
    assert!(!store.schedule_directory().exists());
    std::fs::remove_dir_all(directory).unwrap();
}
