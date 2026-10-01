use super::super::tests::test_store;
use super::*;

#[tokio::test]
async fn imports_once() {
    let (directory, store) = test_store();
    std::fs::write(
        directory.path().join("work.ics"),
        "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\nDTSTART:20260823T093000\nRRULE:FREQ=DAILY\nEND:VEVENT\nEND:VCALENDAR\n",
    )
    .unwrap();

    store
        .import_schedules(&[directory.path().join("work.ics")])
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
}

#[tokio::test]
async fn combines_calendars() {
    let (directory, store) = test_store();
    let sources = directory.path().join("sources");
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
}

#[tokio::test]
async fn rejects_duplicate_sources() {
    let (directory, store) = test_store();
    let first_directory = directory.path().join("first");
    let second_directory = directory.path().join("second");
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
}

#[tokio::test]
async fn validates_sources() {
    let (directory, store) = test_store();
    let first = directory.path().join("first.ics");
    let second = directory.path().join("second.ics");
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
}

#[tokio::test]
async fn reads_existing_sources() {
    let (_directory, store) = test_store();
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
}

#[tokio::test]
async fn reconciles_notion() {
    let (_directory, store) = test_store();
    let date = "2026-09-07";
    store.create(date, "Manual", None).await.unwrap();
    let mut remote = Item {
        id: "notion:view:page".into(),
        text: "Remote".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
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
        .replace_remote(
            date,
            "notion:",
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .update(date, &remote.id, "Local edit", Some("Local notes"))
            .await,
        Err(Error::RemoteItem)
    ));
    store.set_completed(date, &remote.id, true).await.unwrap();
    remote.text = "Renamed remotely".into();
    let refreshed = store
        .replace_remote(
            date,
            "notion:",
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.items[0].text, "Manual");
    assert_eq!(refreshed.items[1].text, "Renamed remotely");
    assert!(refreshed.items[1].completed);
    store.set_completed(date, &remote.id, false).await.unwrap();
    store.delete(date, &remote.id).await.unwrap();
    assert_eq!(
        store
            .replace_remote(
                date,
                "notion:",
                vec![remote],
                store.calendar_lock().await.unwrap()
            )
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        store
            .replace_remote(
                date,
                "notion:",
                vec![],
                store.calendar_lock().await.unwrap()
            )
            .await
            .unwrap()
            .items[0]
            .text,
        "Manual"
    );
}

#[tokio::test]
async fn locks_calendar() {
    let (_directory, first) = test_store();
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
}

#[test]
fn retains_calendar_lock() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (_directory, store) = test_store();
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
            let mut commit =
                std::pin::pin!(store.replace_remote("2026-09-07", "notion:", vec![], guard));
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
    });
}

#[tokio::test]
async fn reuses_calendar_snapshot() {
    // This URL cannot reach the provider: a cache miss must fail validation.
    let configuration = vesper_credentials::NotionCalendar {
        view_url: "invalid".into(),
    };
    let item = Item {
        id: "notion:view:page".into(),
        text: "Conference".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
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

#[tokio::test]
async fn notion_preserves_order() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    let date = "2026-09-12";
    let manual = store
        .create(date, "Manual", None)
        .await
        .unwrap()
        .items
        .remove(0);
    let mut remote = manual.clone();
    remote.id = "notion:first".into();
    remote.text = "Remote".into();
    store
        .replace_remote(
            date,
            "notion:",
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    store
        .reorder(date, vec![remote.id.clone(), manual.id.clone()])
        .await
        .unwrap();
    remote.text = "Updated remotely".into();
    let mut new = remote.clone();
    new.id = "notion:new".into();
    let refreshed = store
        .replace_remote(
            date,
            "notion:",
            vec![new.clone(), remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.items, vec![remote, manual, new]);
}

#[tokio::test]
async fn rollover_consolidates() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    let remote = Item {
        id: "notion:multi".into(),
        text: "Multi-day event".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
        details: None,
    };
    for date in ["2026-09-12", "2026-09-13", "2026-09-17"] {
        store
            .replace_remote(
                date,
                "notion:",
                vec![remote.clone()],
                store.calendar_lock().await.unwrap(),
            )
            .await
            .unwrap();
    }
    for date in ["2026-09-12", "2026-09-13"] {
        store.set_rollover(date, &remote.id, true).await.unwrap();
    }
    let refreshed = store
        .replace_remote(
            "2026-09-12",
            "notion:",
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert!(refreshed.items[0].rollover);
    let changed = store.roll_over("2026-09-15").await.unwrap();
    assert!(changed.contains(&"2026-09-17".into()));
    let moved = store.list("2026-09-15").await.unwrap();
    assert_eq!(moved.items.len(), 1);
    assert!(moved.items[0].id.starts_with("rollover:"));
    for date in ["2026-09-12", "2026-09-13", "2026-09-17"] {
        assert!(store.list(date).await.unwrap().items.is_empty());
        assert!(
            store
                .replace_remote(
                    date,
                    "notion:",
                    vec![remote.clone()],
                    store.calendar_lock().await.unwrap()
                )
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    let refreshed = store
        .replace_remote(
            "2026-09-15",
            "notion:",
            vec![remote],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.items, moved.items);
}

#[tokio::test]
async fn rollover_keeps_history() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    let remote = Item {
        id: "notion:completed-event".into(),
        text: "Finished event".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
        details: None,
    };
    for date in ["2026-09-12", "2026-09-13"] {
        store
            .replace_remote(
                date,
                "notion:",
                vec![remote.clone()],
                store.calendar_lock().await.unwrap(),
            )
            .await
            .unwrap();
    }
    store
        .set_rollover("2026-09-12", &remote.id, true)
        .await
        .unwrap();
    store
        .set_completed("2026-09-13", &remote.id, true)
        .await
        .unwrap();
    store.roll_over("2026-09-14").await.unwrap();
    assert!(store.list("2026-09-14").await.unwrap().items.is_empty());
    assert!(store.list("2026-09-12").await.unwrap().items.is_empty());
    assert!(store.get("2026-09-13", &remote.id).await.unwrap().completed);
    let refreshed = store
        .replace_remote(
            "2026-09-13",
            "notion:",
            vec![remote],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.items.len(), 1);
    assert!(refreshed.items[0].completed);
}

#[tokio::test]
async fn codex_preserves_local_state() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    let date = "2026-09-12";
    store.create(date, "Local task", None).await.unwrap();
    let remote = Item {
        id: "codex:reset-1".into(),
        text: "Codex usage reset".into(),
        description: Some("Original announcement".into()),
        completed: false,
        rollover: false,
        source_owned: true,
        details: Some(Details {
            calendar: "Codex Resets".into(),
            start_date: date.into(),
            start_time: Some("09:09".into()),
            end_date: None,
            end_time: None,
            location: None,
        }),
    };
    store
        .replace_remote(
            date,
            "codex:",
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    store.set_completed(date, &remote.id, true).await.unwrap();
    let mut revised = remote.clone();
    revised.description = Some("Updated announcement".into());
    let list = store
        .replace_remote(
            date,
            "codex:",
            vec![revised],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.items.len(), 2);
    assert_eq!(list.items[0].text, "Local task");
    assert!(list.items[1].completed);
    assert_eq!(
        list.items[1].description.as_deref(),
        Some("Updated announcement")
    );
    assert!(matches!(
        store.update(date, &remote.id, "Edited", None).await,
        Err(Error::CompletedItem)
    ));
    store.set_completed(date, &remote.id, false).await.unwrap();
    store.delete(date, &remote.id).await.unwrap();
    let list = store
        .replace_remote(
            date,
            "codex:",
            vec![remote],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.items.len(), 1);
    let notion = Item {
        id: "notion:other".into(),
        text: "Notion event".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
        details: None,
    };
    store
        .replace_remote(
            date,
            "notion:",
            vec![notion],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    let list = store
        .replace_remote(date, "codex:", vec![], store.calendar_lock().await.unwrap())
        .await
        .unwrap();
    assert_eq!(list.items.len(), 2);
    assert_eq!(list.items[1].id, "notion:other");
}

#[tokio::test]
async fn codex_rollover_is_unique() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    let remote = Item {
        id: "codex:reset".into(),
        text: "Codex usage reset".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
        details: None,
    };
    store
        .replace_remote(
            "2026-09-12",
            "codex:",
            vec![remote.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    store
        .set_rollover("2026-09-12", &remote.id, true)
        .await
        .unwrap();
    store.roll_over("2026-09-14").await.unwrap();
    let source = store
        .replace_remote(
            "2026-09-12",
            "codex:",
            vec![remote],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert!(source.items.is_empty());
    assert_eq!(store.list("2026-09-14").await.unwrap().items.len(), 1);
}

#[tokio::test]
async fn isolates_source_failures() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join(vesper_database::FILE_NAME));
    let date = "2026-09-12";
    let codex = Item {
        id: "codex:reset".into(),
        text: "Codex usage reset".into(),
        description: None,
        completed: false,
        rollover: false,
        source_owned: true,
        details: None,
    };
    store
        .replace_remote(
            date,
            "codex:",
            vec![codex.clone()],
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    let notion = Item {
        id: "notion:event".into(),
        text: "Notion event".into(),
        ..codex.clone()
    };
    let list = store
        .reconcile_calendar(
            date,
            Ok(vec![notion.clone()]),
            Err(Error::Codex("request returned HTTP 503".into())),
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.items, vec![codex.clone(), notion.clone()]);
    assert!(list.sync_error.unwrap().contains("503"));
    let list = store
        .reconcile_calendar(
            date,
            Err(Error::Notion("offline".into())),
            Ok(vec![]),
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.items, vec![notion]);
    assert!(list.sync_error.unwrap().contains("offline"));
    let list = store
        .reconcile_calendar(
            date,
            Ok(vec![]),
            Ok(vec![]),
            store.calendar_lock().await.unwrap(),
        )
        .await
        .unwrap();
    assert!(list.items.is_empty());
    assert!(list.sync_error.is_none());
}

#[tokio::test]
async fn codex_setting_is_local() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(vesper_database::FILE_NAME);
    let store = Store::new(path.clone());
    assert!(!store.read_codex().await.unwrap().enabled);
    store
        .save_codex(Subscription { enabled: true })
        .await
        .unwrap();
    let reopened = Store::new(path);
    assert!(reopened.read_codex().await.unwrap().enabled);
    reopened
        .save_codex(Subscription { enabled: false })
        .await
        .unwrap();
    assert!(!store.read_codex().await.unwrap().enabled);
}

#[tokio::test]
async fn codex_save_waits_for_sync() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(vesper_database::FILE_NAME);
    let store = Store::new(path.clone());
    let lock = store.calendar_lock().await.unwrap();
    let writer = Store::new(path);
    let pending =
        tokio::spawn(async move { writer.save_codex(Subscription { enabled: true }).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!pending.is_finished());
    drop(lock);
    pending.await.unwrap().unwrap();
    assert!(store.read_codex().await.unwrap().enabled);
}

#[tokio::test]
async fn retains_codex_lock() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(vesper_database::FILE_NAME);
    let store = Store::new(path.clone());
    store
        .save_codex(Subscription { enabled: false })
        .await
        .unwrap();
    let mut connection = vesper_database::open(&path).unwrap();
    connection.batch_execute("BEGIN IMMEDIATE").unwrap();
    let competing = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.with_extension("calendar.lock"))
        .unwrap();
    let writer = Store::new(path);
    let pending =
        tokio::spawn(async move { writer.save_codex(Subscription { enabled: true }).await });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!pending.is_finished());
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    let locked = competing.try_lock();
    connection.batch_execute("COMMIT").unwrap();
    drop(connection);
    let lock = store.calendar_lock().await.unwrap();
    assert!(matches!(locked, Err(std::fs::TryLockError::WouldBlock)));
    assert!(store.read_codex().await.unwrap().enabled);
    drop(lock);
}

#[tokio::test]
async fn ics_tasks_own_local_content() {
    let (directory, store) = test_store();
    let date = "2026-09-30";
    let source = directory.path().join("work.ics");
    let original = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:review\nSUMMARY:Original\nDESCRIPTION:Source notes\nLOCATION:Room one\nDTSTART:20260930T093000\nEND:VEVENT\nEND:VCALENDAR\n";
    std::fs::write(&source, original).unwrap();
    store
        .import_schedules(std::slice::from_ref(&source))
        .await
        .unwrap();
    let first = store.sync_schedule(date).await.unwrap();
    let id = &first.items[0].id;
    let metadata = first.items[0].details.clone();
    let edited = store
        .update(date, id, "Local title", Some("Local notes"))
        .await
        .unwrap();
    assert_eq!(edited.items[0].details, metadata);
    std::fs::write(&source, original.replace("Original", "Source changed")).unwrap();
    store
        .import_schedules(std::slice::from_ref(&source))
        .await
        .unwrap();
    let refreshed = store.sync_schedule(date).await.unwrap();
    assert_eq!(refreshed.items[0].text, "Local title");
    assert_eq!(
        refreshed.items[0].description.as_deref(),
        Some("Local notes")
    );
    assert!(!refreshed.items[0].source_owned);
    let reopened = Store::new(store.database_path().to_owned());
    assert_eq!(reopened.get(date, id).await.unwrap(), refreshed.items[0]);
    store.set_completed(date, id, true).await.unwrap();
    assert!(matches!(
        store.update(date, id, "Blocked", None).await,
        Err(Error::CompletedItem)
    ));
    assert!(matches!(
        store.delete(date, id).await,
        Err(Error::CompletedItem)
    ));
    store.set_completed(date, id, false).await.unwrap();
    store.set_rollover(date, id, true).await.unwrap();
    store.roll_over("2026-10-01").await.unwrap();
    store
        .update("2026-10-01", id, "Follow-up", None)
        .await
        .unwrap();
    store.delete("2026-10-01", id).await.unwrap();
    assert!(store.sync_schedule(date).await.unwrap().items.is_empty());
}

#[tokio::test]
async fn remote_followups_become_editable() {
    let (_directory, store) = test_store();
    let date = "2026-09-30";
    for prefix in ["notion:", "codex:"] {
        let remote = Item {
            id: format!("{prefix}review"),
            text: "Remote".into(),
            description: Some("Source notes".into()),
            completed: false,
            rollover: false,
            source_owned: true,
            details: Some(Details {
                calendar: "Source".into(),
                start_date: date.into(),
                start_time: None,
                end_date: None,
                end_time: None,
                location: None,
            }),
        };
        store
            .replace_remote(
                date,
                prefix,
                vec![remote.clone()],
                store.calendar_lock().await.unwrap(),
            )
            .await
            .unwrap();
        assert!(store.get(date, &remote.id).await.unwrap().source_owned);
        assert!(matches!(
            store.update(date, &remote.id, "Blocked", None).await,
            Err(Error::RemoteItem)
        ));
        store.set_rollover(date, &remote.id, true).await.unwrap();
        store.roll_over("2026-10-01").await.unwrap();
        let followup = store.list("2026-10-01").await.unwrap().items.pop().unwrap();
        assert!(followup.id.starts_with("rollover:"));
        assert_eq!(followup.details, remote.details);
        assert!(!followup.source_owned);
        store
            .update(
                "2026-10-01",
                &followup.id,
                "Local follow-up",
                Some("My notes"),
            )
            .await
            .unwrap();
        store
            .replace_remote(
                date,
                prefix,
                vec![remote],
                store.calendar_lock().await.unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            store.get("2026-10-01", &followup.id).await.unwrap().text,
            "Local follow-up"
        );
        assert!(store.list(date).await.unwrap().items.is_empty());
        store.delete("2026-10-01", &followup.id).await.unwrap();
    }
}
