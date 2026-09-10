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
async fn stale_stop_does_not_stop_the_selected_track() {
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
async fn seek_while_paused_preserves_pause() {
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
async fn dropping_spotify_runtime_releases_player() {
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
async fn repeated_loads_ignore_old_request_events() {
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
