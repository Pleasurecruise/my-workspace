use super::*;

#[test]
fn shared_paths() {
    let store = Store::shared().unwrap();
    assert_eq!(
        store.database_path(),
        vesper_database::shared_path().unwrap()
    );
    assert_eq!(
        store.schedule_directory(),
        dirs::data_dir().unwrap().join("me.you-find.vesper/ics"),
    );
}

#[tokio::test]
async fn lists_while_writing() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    store.create("2026-09-07", "Committed", None).await.unwrap();
    let mut writer = vesper_database::open(store.database_path()).unwrap();
    writer
        .batch_execute("BEGIN IMMEDIATE; UPDATE todo_items SET text = 'Pending';")
        .unwrap();
    let result = store.list("2026-09-07").await;
    writer.batch_execute("ROLLBACK;").unwrap();
    assert_eq!(result.unwrap().items[0].text, "Committed");
}

fn test_store() -> (PathBuf, Store) {
    let directory = std::env::temp_dir().join(format!("vesper-todo-{}", uuid::Uuid::new_v4()));
    (
        directory.clone(),
        Store::new(directory.join(vesper_database::FILE_NAME)),
    )
}

#[tokio::test]
async fn handles_crud() {
    let (directory, store) = test_store();
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
    assert!(store.delete(date, &id).await.unwrap().items.is_empty());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn isolates_dates() {
    let (directory, store) = test_store();
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
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn reloads_before_mutation() {
    let (directory, first) = test_store();
    let second = Store::new(directory.join(vesper_database::FILE_NAME));
    first.create("2026-08-23", "First", None).await.unwrap();
    second.create("2026-08-23", "Second", None).await.unwrap();
    assert_eq!(first.list("2026-08-23").await.unwrap().items.len(), 2);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn rolls_back_failed_mutation() {
    use diesel::connection::SimpleConnection;
    let (directory, store) = test_store();
    let original = store.create("2026-08-23", "Keep me", None).await.unwrap();
    let mut connection = vesper_database::open(store.database_path()).unwrap();
    connection.batch_execute("CREATE TRIGGER reject_todo BEFORE INSERT ON todo_items BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(store.create("2026-08-23", "Fail", None).await.is_err());
    assert_eq!(store.list("2026-08-23").await.unwrap(), original);
    drop(connection);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn serializes_writers() {
    let (directory, first) = test_store();
    let second = Store::new(directory.join(vesper_database::FILE_NAME));
    let (first_result, second_result) = tokio::join!(
        first.create("2026-08-23", "First", None),
        second.create("2026-08-23", "Second", None)
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
    assert!(directory.join(vesper_database::FILE_NAME).exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn rejects_long_text() {
    let (directory, store) = test_store();
    let error = store
        .create("2026-08-23", &"x".repeat(MAX_TEXT_LENGTH + 1), None)
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
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("work.ics"),
        "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\nDTSTART:20260823T093000\nRRULE:FREQ=DAILY\nEND:VEVENT\nEND:VCALENDAR\n",
    )
    .unwrap();

    store
        .import_schedules(&[directory.join("work.ics")])
        .await
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
    store
        .create("2026-08-23", "09:30 Standup", None)
        .await
        .unwrap();
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
async fn rejects_duplicate_sources() {
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
    assert!(!store.database_path().exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn validates_sources() {
    let (directory, store) = test_store();
    std::fs::create_dir_all(&directory).unwrap();
    let first = directory.join("first.ics");
    let second = directory.join("second.ics");
    std::fs::write(&first, "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:old\nSUMMARY:Original\nDTSTART:20260823\nEND:VEVENT\nEND:VCALENDAR\n").unwrap();
    store
        .import_schedules(std::slice::from_ref(&first))
        .await
        .unwrap();
    std::fs::write(&first, "BEGIN:VCALENDAR\nEND:VCALENDAR\n").unwrap();
    std::fs::write(&second, "invalid").unwrap();
    assert!(matches!(
        store.import_schedules(&[first, second]).await,
        Err(Error::ScheduleParse { .. })
    ));
    assert_eq!(
        store.sync_schedule("2026-08-23").await.unwrap().items[0].text,
        "Original"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn reads_existing_sources() {
    let (directory, store) = test_store();
    std::fs::create_dir_all(store.schedule_directory()).unwrap();
    let path = store.schedule_directory().join("Existing.ics");
    let content = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:event\nSUMMARY:Existing calendar\nDTSTART:20260823\nEND:VEVENT\nEND:VCALENDAR\n";
    std::fs::write(&path, content).unwrap();
    let list = store.sync_schedule("2026-08-23").await.unwrap();
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].text, "Existing calendar");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    assert_eq!(
        store.sync_schedule("2026-08-23").await.unwrap().items.len(),
        1
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn reconciles_notion() {
    let (directory, store) = test_store();
    let date = "2026-09-07";
    store.create(date, "Manual", None).await.unwrap();
    let mut remote = Item {
        id: "notion:view:page".into(),
        text: "Remote".into(),
        description: None,
        completed: false,
        details: Some(Details {
            calendar: "Notion · Work".into(),
            start_date: date.into(),
            start_time: None,
            end_date: None,
            end_time: None,
            location: None,
        }),
    };
    store
        .replace_notion(
            date,
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .update(date, &remote.id, "Local edit", Some("Local notes"))
            .await,
        Err(Error::ImportedItem)
    ));
    store.set_completed(date, &remote.id, true).await.unwrap();
    remote.text = "Renamed remotely".into();
    let refreshed = store
        .replace_notion(
            date,
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.items[0].text, "Manual");
    assert_eq!(refreshed.items[1].text, "Renamed remotely");
    assert!(refreshed.items[1].completed);
    store.delete(date, &remote.id).await.unwrap();
    assert_eq!(
        store
            .replace_notion(date, vec![remote], store.calendar_lock().await.unwrap())
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        store
            .replace_notion(date, vec![], store.calendar_lock().await.unwrap())
            .await
            .unwrap()
            .items[0]
            .text,
        "Manual"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn locks_calendar() {
    let (directory, first) = test_store();
    let second = Store::new(first.database_path().to_owned());
    let guard = first.calendar_lock().await.unwrap();
    let mut waiting = Box::pin(second.calendar_lock());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    drop(guard);
    let acquired = tokio::time::timeout(std::time::Duration::from_secs(1), waiting)
        .await
        .unwrap()
        .unwrap();
    drop(acquired);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancelled_calendar_commit_retains_lock() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (directory, store) = test_store();
        let guard = store.calendar_lock().await.unwrap();
        let competing = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(store.database_path().with_extension("calendar.lock"))
            .unwrap();
        let (release, blocked) = std::sync::mpsc::channel();
        let blocker = tokio::task::spawn_blocking(move || blocked.recv().unwrap());
        // The only blocking thread is occupied, so the commit remains queued.
        {
            let mut commit = std::pin::pin!(store.replace_notion("2026-09-07", vec![], guard));
            tokio::select! {
                biased;
                _ = &mut commit => panic!("queued commit completed"),
                _ = tokio::task::yield_now() => {},
            }
        }
        let locked = competing.try_lock();
        release.send(()).unwrap();
        blocker.await.unwrap();
        // This read queues behind the detached commit on the same blocking thread.
        store.list("2026-09-07").await.unwrap();
        assert!(matches!(locked, Err(std::fs::TryLockError::WouldBlock)));
        competing.try_lock().unwrap();
        drop(competing);
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[tokio::test]
async fn descriptions_persist_and_edit_preserves_completion_and_date() {
    let (directory, store) = test_store();
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
    store.set_completed(date, id, true).await.unwrap();
    let list = store
        .update(date, id, "Read more", Some("Chapter two"))
        .await
        .unwrap();
    assert!(list.items[0].completed);
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
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn calendar_snapshot_serves_other_dates_without_provider_io() {
    // This URL cannot reach the provider: a cache miss must fail validation.
    let configuration = vesper_credentials::NotionCalendar {
        view_url: "invalid".into(),
    };
    let item = Item {
        id: "notion:view:page".into(),
        text: "Conference".into(),
        description: None,
        completed: false,
        details: Some(Details {
            calendar: "Work".into(),
            start_date: "2026-09-07".into(),
            end_date: Some("2026-09-09".into()),
            start_time: None,
            end_time: None,
            location: None,
        }),
    };
    let mut cache = Some(CalendarSnapshot {
        view_url: configuration.view_url.clone(),
        loaded: std::time::Instant::now(),
        items: vec![item],
    });
    for date in ["2026-09-07", "2026-09-09", "2026-09-08", "2026-09-07"] {
        assert_eq!(
            read_notion(&mut cache, &configuration, date, false)
                .await
                .unwrap()
                .len(),
            1
        );
    }
    assert!(
        read_notion(&mut cache, &configuration, "2026-09-10", false)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        read_notion(&mut cache, &configuration, "2026-09-07", true)
            .await
            .is_err()
    );
    assert_eq!(cache.as_ref().unwrap().items.len(), 1);
    let changed = vesper_credentials::NotionCalendar {
        view_url: "another-invalid-view".into(),
    };
    assert!(
        read_notion(&mut cache, &changed, "2026-09-07", false)
            .await
            .is_err()
    );
    cache.as_mut().unwrap().loaded =
        std::time::Instant::now() - std::time::Duration::from_secs(301);
    assert!(
        read_notion(&mut cache, &configuration, "2026-09-07", false)
            .await
            .is_err()
    );
}
