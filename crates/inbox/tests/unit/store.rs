use super::*;

#[tokio::test]
async fn isolates_corrupt_notification_storage() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notifications.sqlite3");
    std::fs::write(&path, b"{").unwrap();
    let state = Store::new(path.clone());
    assert!(state.snapshot.read().await.is_err());
    assert!(state.mark_read("message-1").await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{");
}

#[tokio::test]
async fn failed_replacement_preserves_notification_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notifications.sqlite3");
    let state = Store::new(path.clone());
    state
        .accept(Message {
            id: "message-1".to_owned(),
            time: 1,
            event: "message".to_owned(),
            topic: TOPIC.to_owned(),
            title: None,
            message: Some("Message".to_owned()),
            tags: vec![],
        })
        .await
        .unwrap();
    let saved = std::fs::read(&path).unwrap();
    let retained = directory.path().join("retained.sqlite3");
    std::fs::rename(&path, &retained).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(state.mark_read("message-1").await.is_err());
    assert_eq!(
        state
            .snapshot
            .read()
            .await
            .as_ref()
            .unwrap()
            .notifications
            .len(),
        1
    );
    assert_eq!(std::fs::read(retained).unwrap(), saved);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[tokio::test]
async fn deduplicates_messages() {
    let path = std::env::temp_dir().join(format!(
        "vesper-notifications-{}.sqlite3",
        std::process::id()
    ));
    drop(std::fs::remove_file(&path));
    let state = Store::new(path.clone());
    let message = Message {
        id: "message-1".to_owned(),
        time: 1,
        event: "message".to_owned(),
        topic: "mail-summary".to_owned(),
        title: Some("Mail".to_owned()),
        message: Some("Summary".to_owned()),
        tags: vec![],
    };
    assert!(state.accept(message).await.unwrap().is_some());
    let duplicate = Message {
        id: "message-1".to_owned(),
        time: 2,
        event: "message".to_owned(),
        topic: "mail-summary".to_owned(),
        title: None,
        message: Some("Duplicate".to_owned()),
        tags: vec![],
    };
    assert!(state.accept(duplicate).await.unwrap().is_none());
    assert_eq!(
        state
            .snapshot
            .read()
            .await
            .as_ref()
            .unwrap()
            .notifications
            .len(),
        1
    );
    drop(std::fs::remove_file(path));
}

#[tokio::test]
async fn removes_read_message() {
    let path = std::env::temp_dir().join(format!(
        "vesper-read-notifications-{}.sqlite3",
        std::process::id()
    ));
    let message = Message {
        id: "message-1".to_owned(),
        time: 1,
        event: "message".to_owned(),
        topic: TOPIC.to_owned(),
        title: None,
        message: Some("Message".to_owned()),
        tags: vec![],
    };
    let state = Store::new(path.clone());
    state.accept(message).await.unwrap();
    assert!(state.mark_read("message-1").await.unwrap().is_empty());

    let restored = Store::new(path.clone());
    assert!(
        restored
            .snapshot
            .read()
            .await
            .as_ref()
            .unwrap()
            .notifications
            .is_empty()
    );
    drop(std::fs::remove_file(path));
}
