use super::*;

#[tokio::test(start_paused = true)]
async fn deadline_retires_worker() {
    let (commands, receiver) = mpsc::channel();
    let retired = Arc::new(AtomicBool::new(false));
    let audio = AudioPlayer {
        worker: Mutex::new(Some(Worker {
            commands,
            output: Arc::new(Mutex::new(None)),
            retired: Arc::clone(&retired),
        })),
    };
    let changes = watch::channel(0).0;
    assert!(
        audio
            .load(Vec::new(), track(), 0, changes.subscribe())
            .await
            .unwrap_err()
            .to_string()
            .contains("timed out")
    );
    assert!(retired.load(Ordering::SeqCst));
    assert!(audio.worker.lock().unwrap().is_none());
    drop(receiver);
    tokio::time::resume();
    // A retry uses a fresh worker. Invalid audio fails immediately rather than hanging again.
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        audio.load(Vec::new(), track(), 0, changes.subscribe()),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains("could not be decoded"));
}

fn track() -> Track {
    Track {
        id: "current".to_owned(),
        name: "Current".to_owned(),
        artists: Vec::new(),
        album: String::new(),
        duration_ms: 1000,
        added_at: String::new(),
        cover_key: None,
    }
}

#[tokio::test]
async fn stopped_worker_can_be_replaced() {
    let (commands, receiver) = mpsc::channel();
    drop(receiver);
    let retired = Arc::new(AtomicBool::new(false));
    let audio = AudioPlayer {
        worker: Mutex::new(Some(Worker {
            commands,
            output: Arc::new(Mutex::new(None)),
            retired: Arc::clone(&retired),
        })),
    };
    let changes = watch::channel(0).0;
    assert!(
        audio
            .load(Vec::new(), track(), 0, changes.subscribe())
            .await
            .unwrap_err()
            .to_string()
            .contains("thread stopped")
    );
    assert!(retired.load(Ordering::SeqCst));
    assert!(
        audio
            .load(Vec::new(), track(), 0, changes.subscribe())
            .await
            .unwrap_err()
            .to_string()
            .contains("could not be decoded")
    );
}

#[test]
fn snapshot_preserves_seek_offset_and_loaded_track() {
    let (sink, _source) = Sink::new();
    sink.pause();
    sink.append(rodio::buffer::SamplesBuffer::new(
        1,
        44100,
        vec![0.0_f32; 44100],
    ));
    let (commands, _receiver) = mpsc::channel();
    let audio = AudioPlayer {
        worker: Mutex::new(Some(Worker {
            commands,
            output: Arc::new(Mutex::new(Some(Output {
                sink,
                offset: Duration::from_millis(500),
                track: track(),
            }))),
            retired: Arc::new(AtomicBool::new(false)),
        })),
    };
    let snapshot = audio.snapshot().unwrap();
    assert_eq!(snapshot.progress_ms, 500);
    assert_eq!(snapshot.track.unwrap().id, "current");
    assert!(!snapshot.playing);
    assert!(!snapshot.ended);
    audio.resume().unwrap();
    assert!(audio.snapshot().unwrap().playing);
    audio.retire(None);
    assert!(audio.snapshot().unwrap().ended);
}

#[test]
fn cancelled_audio_is_not_installed() {
    let (commands, receiver) = mpsc::channel();
    let (response, result) = oneshot::channel();
    let changes = watch::channel(1).0;
    commands
        .send(Command::Load {
            bytes: Vec::new(),
            track: track(),
            generation: 0,
            changes: changes.subscribe(),
            response,
        })
        .unwrap();
    drop(commands);
    let output = Arc::new(Mutex::new(None));
    run(
        receiver,
        Arc::clone(&output),
        Arc::new(AtomicBool::new(false)),
    );
    // A stale command is discarded before even attempting to decode its invalid bytes.
    assert!(result.blocking_recv().is_err());
    assert!(output.lock().unwrap().is_none());
}
