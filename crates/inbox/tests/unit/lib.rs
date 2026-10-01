use super::*;

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
    let path = directory.path().join("notifications.sqlite3");
    let state = Arc::new(Store::new(path.clone()));
    let guard = state.snapshot.write().await;
    let mut subscription = Subscription::default();
    let worker = state.clone();
    let (started, waiting) = tokio::sync::oneshot::channel();
    subscription
        .update(Some(true), |mut stop| async {
            Ok(Some(tokio::spawn(async move {
                started.send(()).unwrap();
                worker
                    .accept(Message {
                        id: "accepted".to_owned(),
                        time: 1,
                        event: "message".to_owned(),
                        topic: store::TOPIC.to_owned(),
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
    let restored = Store::new(path);
    let disk = restored.snapshot.read().await;
    let disk = disk.as_ref().unwrap();
    let memory = state.snapshot.read().await;
    let memory = memory.as_ref().unwrap();
    assert_eq!(disk.notifications.len(), 1);
    assert_eq!(disk.last_id, memory.last_id);
    assert_eq!(memory.notifications[0].id, "accepted");
}

#[tokio::test]
async fn stream_accepts_only_valid_data_lines() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("inbox.sqlite3"));
    let updates = Arc::new(std::sync::Mutex::new(Vec::new()));
    let received = updates.clone();
    let listener: Listener = Arc::new(move |notifications: Vec<Notification>| {
        received.lock().unwrap().push(notifications.len());
    });
    let body = concat!(
        ": keepalive comment\n",
        "event: message\n",
        "data: not json\n",
        "data: {\"id\":\"a\",\"time\":1,\"event\":\"message\",\"topic\":\"mail-summary\",\"message\":\"First\"}\r\n",
        "data:{\"id\":\"b\",\"time\":2,\"event\":\"message\",\"topic\":\"mail-summary\",\"message\":\"Second\"}\n",
        "data: {\"id\":\"partial\"",
    );
    let response = reqwest::Response::from(http::Response::new(body.to_owned()));
    let (_sender, mut stop) = watch::channel(());
    consume(&store, &listener, response, &mut stop).await;
    assert_eq!(*updates.lock().unwrap(), [1, 2]);
    let snapshot = store.snapshot.read().await;
    assert_eq!(snapshot.as_ref().unwrap().last_id.as_deref(), Some("b"));
}
