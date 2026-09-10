use super::*;

#[tokio::test]
async fn provider_selection_cancels_playback_before_it_reaches_the_runtime() {
    let actions = PlaybackActions::default();
    let spotify_playing = AtomicBool::new(false);
    let qq_playing = AtomicBool::new(false);
    let (_release, blocked) = tokio::sync::oneshot::channel::<()>();
    let older = actions.run(async {
        // Model an old IPC waiting in pause_inactive/runtime creation, before Spotify::play.
        let _ = blocked.await;
        spotify_playing.store(true, Ordering::SeqCst);
        CommandResponse::Ready { data: () }
    });
    tokio::pin!(older);
    assert!(futures_util::poll!(&mut older).is_pending());
    let newer = actions.run(async {
        spotify_playing.store(false, Ordering::SeqCst);
        qq_playing.store(true, Ordering::SeqCst);
        CommandResponse::Ready { data: () }
    });
    let (old, new) = tokio::join!(older, newer);
    assert!(matches!(old, CommandResponse::Failed { .. }));
    assert!(matches!(new, CommandResponse::Ready { .. }));
    assert!(!spotify_playing.load(Ordering::SeqCst));
    assert!(qq_playing.load(Ordering::SeqCst));
}

#[tokio::test]
async fn failed_commands_release_the_action_for_retry() {
    let actions = PlaybackActions::default();
    let failed = actions
        .run(async {
            CommandResponse::<()>::Failed {
                message: "Unavailable".to_owned(),
            }
        })
        .await;
    assert!(matches!(failed, CommandResponse::Failed { message } if message == "Unavailable"));
    assert!(matches!(
        actions
            .run(async { CommandResponse::Ready { data: 42 } })
            .await,
        CommandResponse::Ready { data: 42 }
    ));
}
