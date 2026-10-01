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

fn message(id: &str, body: &str) -> Message {
    Message {
        id: id.to_owned(),
        time: 1,
        event: "message".to_owned(),
        topic: TOPIC.to_owned(),
        title: Some("Fallback title".to_owned()),
        message: Some(body.to_owned()),
        tags: vec![],
    }
}

#[tokio::test]
async fn projects_envelopes_and_falls_back_to_plain_text() {
    let directory = tempfile::tempdir().unwrap();
    let state = Store::new(directory.path().join("inbox.sqlite3"));
    let envelope = r#"{"source":" Gmail ","title":"Digest","body":"Three new mails"}"#;
    let projected = state.accept(message("a", envelope)).await.unwrap().unwrap();
    assert_eq!(projected[0].source, "Gmail");
    assert_eq!(projected[0].title.as_deref(), Some("Digest"));
    assert_eq!(projected[0].message, "Three new mails");

    let plain = state
        .accept(message("b", "Plain body"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plain[0].source, TOPIC);
    assert_eq!(plain[0].title.as_deref(), Some("Fallback title"));
    assert_eq!(plain[0].message, "Plain body");
    assert_eq!(plain.len(), 2);
}

#[tokio::test]
async fn ignores_control_events_and_rejects_invalid_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let state = Store::new(directory.path().join("inbox.sqlite3"));
    let mut keepalive = message("a", "body");
    keepalive.event = "keepalive".to_owned();
    assert!(state.accept(keepalive).await.unwrap().is_none());
    assert!(state.accept(message("b", "   ")).await.unwrap().is_none());

    let mut topic = message("c", "body");
    topic.topic = "other".to_owned();
    assert!(state.accept(topic).await.is_err());
    let mut time = message("d", "body");
    time.time = -1;
    assert!(state.accept(time).await.is_err());
    let mut tags = message("e", "body");
    tags.tags = vec!["x".repeat(101)];
    assert!(state.accept(tags).await.is_err());
    assert!(
        state
            .accept(message("f", &"x".repeat(500_001)))
            .await
            .is_err()
    );
    assert!(
        state
            .snapshot
            .read()
            .await
            .as_ref()
            .unwrap()
            .notifications
            .is_empty()
    );
}

#[tokio::test]
async fn keeps_the_newest_notifications_within_the_limit() {
    let directory = tempfile::tempdir().unwrap();
    let state = Store::new(directory.path().join("inbox.sqlite3"));
    for index in 0..=LIMIT {
        state
            .accept(message(&format!("m{index}"), "body"))
            .await
            .unwrap();
    }
    let snapshot = state.snapshot.read().await;
    let snapshot = snapshot.as_ref().unwrap();
    assert_eq!(snapshot.notifications.len(), LIMIT);
    assert_eq!(snapshot.notifications[0].id, format!("m{LIMIT}"));
    assert_eq!(
        snapshot.last_id.as_deref(),
        Some(format!("m{LIMIT}").as_str())
    );
}
