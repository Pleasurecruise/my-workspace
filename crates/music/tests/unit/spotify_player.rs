use super::*;

struct SilentSink;
impl audio_backend::Sink for SilentSink {
    fn write(
        &mut self,
        _: librespot_playback::decoder::AudioPacket,
        _: &mut librespot_playback::convert::Converter,
    ) -> audio_backend::SinkResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn ignores_stale_stop() {
    let player = Player::new(
        PlayerConfig::default(),
        Session::new(SessionConfig::default(), None),
        Box::new(NoOpVolume),
        || Box::new(SilentSink),
    );
    let state = Arc::new(RwLock::new(State {
        track_id: Some("new-track".to_owned()),
        playing: true,
        request_id: Some(2),
        ..State::default()
    }));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(PlayerEvent::Stopped {
        play_request_id: 1,
        track_id: track_uri("0000000000000000000000").unwrap(),
    })
    .unwrap();
    drop(tx);
    read_events(
        rx,
        Arc::downgrade(&player),
        Arc::clone(&state),
        Arc::new(RwLock::new(Queue::default())),
    )
    .await;
    assert!(
        state.read().await.playing,
        "an old-track event stopped the newly selected track"
    );
}

#[tokio::test]
async fn seeks_while_paused() {
    let player = Player::new(
        PlayerConfig::default(),
        Session::new(SessionConfig::default(), None),
        Box::new(NoOpVolume),
        || Box::new(SilentSink),
    );
    let state = Arc::new(RwLock::new(State {
        track_id: Some("0000000000000000000000".to_owned()),
        playing: false,
        request_id: Some(1),
        ..State::default()
    }));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(PlayerEvent::Seeked {
        play_request_id: 1,
        track_id: track_uri("0000000000000000000000").unwrap(),
        position_ms: 500,
    })
    .unwrap();
    drop(tx);
    read_events(
        rx,
        Arc::downgrade(&player),
        Arc::clone(&state),
        Arc::new(RwLock::new(Queue::default())),
    )
    .await;
    assert!(
        !state.read().await.playing,
        "a position event incorrectly resumed paused UI state"
    );
}

#[tokio::test]
async fn releases_player() {
    let player = Player::new(
        PlayerConfig::default(),
        Session::new(SessionConfig::default(), None),
        Box::new(NoOpVolume),
        || Box::new(SilentSink),
    );
    let weak = Arc::downgrade(&player);
    let task = tokio::spawn(read_events(
        player.get_player_event_channel(),
        Arc::downgrade(&player),
        Arc::new(RwLock::new(State::default())),
        Arc::new(RwLock::new(Queue::default())),
    ));
    drop(player);
    tokio::task::yield_now().await;
    let retained = weak.upgrade().is_some();
    task.abort();
    let _ = task.await;
    assert!(
        !retained,
        "the event reader retains the old Spotify player after reset"
    );
}

#[tokio::test]
async fn ignores_stale_load_events() {
    let state = Arc::new(RwLock::new(State::default()));
    {
        let mut state = state.write().await;
        state.select("same-track".to_owned(), 1000);
        state.select("same-track".to_owned(), 1000);
    }
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(PlayerEvent::PlayRequestIdChanged { play_request_id: 1 })
        .unwrap();
    tx.send(PlayerEvent::PlayRequestIdChanged { play_request_id: 1 })
        .unwrap();
    tx.send(PlayerEvent::Unavailable {
        play_request_id: 1,
        track_id: track_uri("0000000000000000000000").unwrap(),
    })
    .unwrap();
    tx.send(PlayerEvent::PlayRequestIdChanged { play_request_id: 2 })
        .unwrap();
    tx.send(PlayerEvent::Playing {
        play_request_id: 2,
        track_id: track_uri("0000000000000000000000").unwrap(),
        position_ms: 123,
    })
    .unwrap();
    tx.send(PlayerEvent::EndOfTrack {
        play_request_id: 1,
        track_id: track_uri("0000000000000000000000").unwrap(),
    })
    .unwrap();
    drop(tx);
    read_events(
        rx,
        Weak::new(),
        Arc::clone(&state),
        Arc::new(RwLock::new(Queue::default())),
    )
    .await;
    let state = state.read().await;
    assert!(state.playing);
    assert_eq!(state.progress_ms, 123);
    assert!(state.failure.is_none());
    assert!(!state.ended);
}

#[tokio::test]
async fn paused_end_does_not_advance() {
    let state = Arc::new(RwLock::new(State {
        track_id: Some("current".to_owned()),
        request_id: Some(1),
        order: PlaybackOrder::RepeatOne,
        ..State::default()
    }));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(PlayerEvent::EndOfTrack {
        play_request_id: 1,
        track_id: track_uri("0000000000000000000000").unwrap(),
    })
    .unwrap();
    drop(tx);
    read_events(
        rx,
        Weak::new(),
        Arc::clone(&state),
        Arc::new(RwLock::new(Queue::default())),
    )
    .await;
    let state = state.read().await;
    assert!(state.ended);
    assert!(!state.playing);
    assert!(state.failure.is_none());
}

#[tokio::test]
async fn rejects_playback_account() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    for (status, body, expected) in [
        (200, r#"{"product":"free"}"#, "Premium is required"),
        (200, r#"{"product":"open"}"#, "Premium is required"),
        (200, r#"{}"#, "did not confirm Premium"),
        (200, r#"{"product":"unknown"}"#, "did not confirm Premium"),
        (200, "invalid-json", "request failed"),
        (401, r#"{"message":"synthetic-private-body"}"#, "401"),
        (403, r#"{}"#, "403"),
        (429, r#"{}"#, "429"),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut request = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).await.unwrap();
                if line == "\r\n" {
                    break;
                }
                request.push_str(&line);
            }
            assert!(request.starts_with("GET /me HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer synthetic-playback-token\r\n")
            );
            stream.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        });
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let result = LocalPlayer::connect(
            &http,
            &format!("http://{address}"),
            "synthetic-playback-token".to_owned(),
        )
        .await;
        let error = match result {
            Err(error) => error.to_string(),
            Ok(_) => panic!("unverified account started local playback"),
        };
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("synthetic-private-body"));
        assert!(!error.contains("synthetic-playback-token"));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn exits_on_free_account() {
    const PROBE: &str = "VESPER_TEST_LIBRESPOT_FREE_ACCOUNT";
    if std::env::var_os(PROBE).is_some() {
        let session = Session::new(SessionConfig::default(), None);
        session.set_user_attribute("type", "free");
        panic!("expected librespot to exit");
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "spotify::player::tests::exits_on_free_account"])
        .env(PROBE, "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
