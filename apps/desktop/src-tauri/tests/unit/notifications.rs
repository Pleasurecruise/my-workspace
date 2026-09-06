use super::*;

#[tokio::test]
async fn isolates_corrupt_notification_storage() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notifications.json");
    std::fs::write(&path, b"{").unwrap();
    let state = NotificationState::new(path.clone());
    assert!(state.store.read().await.is_err());
    assert!(state.mark_read("message-1").await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{");
}

#[tokio::test]
async fn failed_replacement_preserves_notification_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notifications.json");
    let state = NotificationState::new(path.clone());
    state
        .accept(NtfyMessage {
            id: "message-1".to_owned(),
            time: 1,
            event: "message".to_owned(),
            topic: NTFY_TOPIC.to_owned(),
            title: None,
            message: Some("Message".to_owned()),
            tags: vec![],
        })
        .await
        .unwrap();
    let saved = std::fs::read(&path).unwrap();
    let retained = directory.path().join("retained.json");
    std::fs::rename(&path, &retained).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(state.mark_read("message-1").await.is_err());
    assert_eq!(
        state
            .store
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
    let path =
        std::env::temp_dir().join(format!("vesper-notifications-{}.json", std::process::id()));
    drop(std::fs::remove_file(&path));
    let state = NotificationState::new(path.clone());
    let message = NtfyMessage {
        id: "message-1".to_owned(),
        time: 1,
        event: "message".to_owned(),
        topic: "mail-summary".to_owned(),
        title: Some("Mail".to_owned()),
        message: Some("Summary".to_owned()),
        tags: vec![],
    };
    assert!(state.accept(message).await.unwrap().is_some());
    let duplicate = NtfyMessage {
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
            .store
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
        "vesper-read-notifications-{}.json",
        std::process::id()
    ));
    let message = NtfyMessage {
        id: "message-1".to_owned(),
        time: 1,
        event: "message".to_owned(),
        topic: NTFY_TOPIC.to_owned(),
        title: None,
        message: Some("Message".to_owned()),
        tags: vec![],
    };
    let state = NotificationState::new(path.clone());
    state.accept(message).await.unwrap();
    assert!(state.mark_read("message-1").await.unwrap().is_empty());

    let restored = NotificationState::new(path.clone());
    assert!(
        restored
            .store
            .read()
            .await
            .as_ref()
            .unwrap()
            .notifications
            .is_empty()
    );
    drop(std::fs::remove_file(path));
}

#[tokio::test]
async fn subscription_only_connects_on_active_route_and_stops_on_exit() {
    let mut subscription = Subscription::default();
    let no_connection = |_| async { panic!("inactive or unchanged route must not connect") };
    subscription.update(None, no_connection).await.unwrap();
    subscription
        .update(Some(false), no_connection)
        .await
        .unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
    subscription
        .update(Some(true), |mut stop| async {
            Ok(Some(tokio::spawn(async move {
                let _receiver = receiver;
                let _ = stop.changed().await;
            })))
        })
        .await
        .unwrap();
    subscription
        .update(Some(true), no_connection)
        .await
        .unwrap();
    subscription
        .update(Some(false), no_connection)
        .await
        .unwrap();
    assert!(subscription.task.is_none());
    assert!(sender.send(()).is_err());
    // Saving credentials after navigation must not reconnect.
    subscription.update(None, no_connection).await.unwrap();
}

#[tokio::test]
async fn credential_restart_stops_old_stream_before_starting_replacement() {
    let mut subscription = Subscription::default();
    let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
    subscription
        .update(Some(true), |mut stop| async {
            Ok(Some(tokio::spawn(async move {
                let _receiver = receiver;
                let _ = stop.changed().await;
            })))
        })
        .await
        .unwrap();
    subscription
        .update(None, |_| async {
            assert!(sender.is_closed());
            Err("Invalid configuration".to_owned())
        })
        .await
        .unwrap_err();
    assert!(subscription.task.is_none());
    subscription
        .update(Some(true), |_| async { Ok(None) })
        .await
        .unwrap();
}

#[tokio::test]
async fn leaving_inbox_waits_for_accepted_message_to_commit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notifications.json");
    let state = std::sync::Arc::new(NotificationState::new(path.clone()));
    let guard = state.store.write().await;
    let mut subscription = Subscription::default();
    let worker = state.clone();
    let (started, waiting) = tokio::sync::oneshot::channel();
    subscription
        .update(Some(true), |mut stop| async {
            Ok(Some(tokio::spawn(async move {
                started.send(()).unwrap();
                worker
                    .accept(NtfyMessage {
                        id: "accepted".to_owned(),
                        time: 1,
                        event: "message".to_owned(),
                        topic: NTFY_TOPIC.to_owned(),
                        title: None,
                        message: Some("Persist before stopping".to_owned()),
                        tags: vec![],
                    })
                    .await
                    .unwrap();
                let _ = stop.changed().await;
            })))
        })
        .await
        .unwrap();
    waiting.await.unwrap();
    let stop = subscription.update(Some(false), |_| async { panic!("must not reconnect") });
    tokio::pin!(stop);
    assert!(futures_util::poll!(&mut stop).is_pending());
    drop(guard);
    stop.await.unwrap();
    let disk: NotificationStore = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let memory = state.store.read().await;
    let memory = memory.as_ref().unwrap();
    assert_eq!(disk.notifications.len(), 1);
    assert_eq!(disk.last_id, memory.last_id);
    assert_eq!(memory.notifications[0].id, "accepted");
}
