use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use tokio::sync::{oneshot, watch};

use crate::{Error, Result, Track};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub(super) struct Snapshot {
    pub track: Option<Track>,
    pub playing: bool,
    pub progress_ms: u64,
    pub ended: bool,
}

#[derive(Default)]
pub(super) struct AudioPlayer {
    worker: Mutex<Option<Worker>>,
}

struct Worker {
    commands: mpsc::Sender<Command>,
    output: Arc<Mutex<Option<Output>>>,
    retired: Arc<AtomicBool>,
}

struct Output {
    sink: Sink,
    offset: Duration,
    track: Track,
}

enum Command {
    Load {
        bytes: Vec<u8>,
        track: Track,
        generation: u64,
        changes: watch::Receiver<u64>,
        response: oneshot::Sender<Result<()>>,
    },
    Seek {
        position: Duration,
        generation: u64,
        changes: watch::Receiver<u64>,
        response: oneshot::Sender<Result<()>>,
    },
}

impl AudioPlayer {
    pub async fn load(
        &self,
        bytes: Vec<u8>,
        track: Track,
        generation: u64,
        changes: watch::Receiver<u64>,
    ) -> Result<()> {
        let (response, result) = oneshot::channel();
        let (sent, retired) = {
            let mut worker = self.worker.lock().map_err(|_| {
                Error::Playback("QQ Music audio worker lock is poisoned".to_owned())
            })?;
            if worker.is_none() {
                *worker = Some(Worker::start()?);
            }
            let worker = worker
                .as_ref()
                .expect("the audio worker was just initialized");
            let sent = worker.commands.send(Command::Load {
                bytes,
                track,
                generation,
                changes,
                response,
            });
            (sent, Arc::clone(&worker.retired))
        };
        if sent.is_err() {
            self.retire(Some(&retired));
            return Err(Error::Playback("QQ Music audio thread stopped".to_owned()));
        }
        match tokio::time::timeout(COMMAND_TIMEOUT, result).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.retire(Some(&retired));
                Err(Error::Playback("QQ Music audio thread stopped".to_owned()))
            }
            Err(_) => {
                self.retire(Some(&retired));
                Err(Error::Playback(
                    "QQ Music audio loading timed out; try playing the track again".to_owned(),
                ))
            }
        }
    }

    pub async fn seek(
        &self,
        position: Duration,
        generation: u64,
        changes: watch::Receiver<u64>,
    ) -> Result<()> {
        let (response, result) = oneshot::channel();
        let (sent, retired) = {
            let worker = self.worker.lock().map_err(|_| {
                Error::Playback("QQ Music audio worker lock is poisoned".to_owned())
            })?;
            let worker = worker
                .as_ref()
                .ok_or_else(|| Error::Playback("No QQ Music song has been loaded".to_owned()))?;
            let sent = worker.commands.send(Command::Seek {
                position,
                generation,
                changes,
                response,
            });
            (sent, Arc::clone(&worker.retired))
        };
        if sent.is_err() {
            self.retire(Some(&retired));
            return Err(Error::Playback("QQ Music audio thread stopped".to_owned()));
        }
        match tokio::time::timeout(COMMAND_TIMEOUT, result).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.retire(Some(&retired));
                Err(Error::Playback("QQ Music audio thread stopped".to_owned()))
            }
            Err(_) => {
                self.retire(Some(&retired));
                Err(Error::Playback(
                    "QQ Music audio seek timed out; try playing the track again".to_owned(),
                ))
            }
        }
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let worker = self
            .worker
            .lock()
            .map_err(|_| Error::Playback("QQ Music audio worker lock is poisoned".to_owned()))?;
        let Some(worker) = worker.as_ref() else {
            return Ok(Snapshot {
                ended: true,
                ..Snapshot::default()
            });
        };
        let output = worker
            .output
            .lock()
            .map_err(|_| Error::Playback("QQ Music audio output lock is poisoned".to_owned()))?;
        Ok(output.as_ref().map_or(
            Snapshot {
                ended: true,
                ..Snapshot::default()
            },
            |output| Snapshot {
                track: Some(output.track.clone()),
                playing: !output.sink.is_paused() && !output.sink.empty(),
                progress_ms: (output.offset + output.sink.get_pos()).as_millis() as u64,
                ended: output.sink.empty(),
            },
        ))
    }

    pub fn pause(&self) -> Result<()> {
        let worker = self
            .worker
            .lock()
            .map_err(|_| Error::Playback("QQ Music audio worker lock is poisoned".to_owned()))?;
        if let Some(worker) = worker.as_ref() {
            let output = worker.output.lock().map_err(|_| {
                Error::Playback("QQ Music audio output lock is poisoned".to_owned())
            })?;
            if let Some(sink) = output.as_ref() {
                sink.sink.pause();
            }
        }
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        let worker = self
            .worker
            .lock()
            .map_err(|_| Error::Playback("QQ Music audio worker lock is poisoned".to_owned()))?;
        let worker = worker
            .as_ref()
            .ok_or_else(|| Error::Playback("No QQ Music song has been loaded".to_owned()))?;
        let output = worker
            .output
            .lock()
            .map_err(|_| Error::Playback("QQ Music audio output lock is poisoned".to_owned()))?;
        let sink = output
            .as_ref()
            .filter(|output| !output.sink.empty())
            .ok_or_else(|| Error::Playback("QQ Music track must be loaded again".to_owned()))?;
        sink.sink.play();
        Ok(())
    }

    // Retiring silences any installed sink and prevents a late native call from installing another.
    // The next load starts a fresh worker; an OS call already executing cannot be forcibly aborted.
    pub fn retire(&self, expected: Option<&Arc<AtomicBool>>) {
        if let Ok(mut worker) = self.worker.lock() {
            let matches = worker.as_ref().is_some_and(|worker| {
                expected.is_none_or(|expected| Arc::ptr_eq(expected, &worker.retired))
            });
            if matches && let Some(worker) = worker.take() {
                worker.retired.store(true, Ordering::SeqCst);
                if let Ok(mut output) = worker.output.lock()
                    && let Some(sink) = output.take()
                {
                    sink.sink.stop();
                }
            }
        }
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.retire(None);
    }
}

impl Worker {
    fn start() -> Result<Self> {
        let (commands, receiver) = mpsc::channel();
        let output = Arc::new(Mutex::new(None));
        let retired = Arc::new(AtomicBool::new(false));
        let worker_output = Arc::clone(&output);
        let worker_retired = Arc::clone(&retired);
        std::thread::Builder::new()
            .name("vesper-qq-music-audio".to_owned())
            .spawn(move || run(receiver, worker_output, worker_retired))
            .map_err(|error| {
                Error::Playback(format!("QQ Music audio thread could not start: {error}"))
            })?;
        Ok(Self {
            commands,
            output,
            retired,
        })
    }
}

fn run(
    receiver: mpsc::Receiver<Command>,
    output: Arc<Mutex<Option<Output>>>,
    retired: Arc<AtomicBool>,
) {
    let mut stream: Option<OutputStream> = None;
    let mut media: Option<Arc<[u8]>> = None;
    while let Ok(command) = receiver.recv() {
        if retired.load(Ordering::SeqCst) {
            return;
        }
        match command {
            Command::Load {
                bytes,
                track,
                generation,
                changes,
                response,
            } => {
                if response.is_closed() || *changes.borrow() != generation {
                    continue;
                }
                let result = (|| {
                    let bytes: Arc<[u8]> = bytes.into();
                    let decoder =
                        Decoder::try_from(Cursor::new(Arc::clone(&bytes))).map_err(|error| {
                            Error::Playback(format!("QQ Music audio could not be decoded: {error}"))
                        })?;
                    if stream.is_none() {
                        let mut next =
                            OutputStreamBuilder::open_default_stream().map_err(|error| {
                                Error::Playback(format!(
                                    "QQ Music audio output is unavailable: {error}"
                                ))
                            })?;
                        next.log_on_drop(false);
                        stream = Some(next);
                    }
                    let current = changes.borrow();
                    let mut output = output.lock().map_err(|_| {
                        Error::Playback("QQ Music audio output lock is poisoned".to_owned())
                    })?;
                    if retired.load(Ordering::SeqCst)
                        || response.is_closed()
                        || *current != generation
                    {
                        return Err(Error::Playback(
                            "QQ Music audio load was cancelled".to_owned(),
                        ));
                    }
                    let stream = stream
                        .as_ref()
                        .expect("the output stream was just initialized");
                    let next = Sink::connect_new(stream.mixer());
                    next.append(decoder);
                    if let Some(previous) = output.replace(Output {
                        sink: next,
                        offset: Duration::ZERO,
                        track,
                    }) {
                        previous.sink.stop();
                    }
                    media = Some(bytes);
                    Ok(())
                })();
                let _ = response.send(result);
            }
            Command::Seek {
                position,
                generation,
                changes,
                response,
            } => {
                if response.is_closed() || *changes.borrow() != generation {
                    continue;
                }
                let result = (|| {
                    let bytes = media.as_ref().ok_or_else(|| {
                        Error::Playback("No QQ Music song has been loaded".to_owned())
                    })?;
                    let mut decoder =
                        Decoder::try_from(Cursor::new(Arc::clone(bytes))).map_err(|error| {
                            Error::Playback(format!("QQ Music audio could not be decoded: {error}"))
                        })?;
                    decoder.try_seek(position).map_err(|error| {
                        Error::Playback(format!("QQ Music seek failed: {error}"))
                    })?;
                    let current = changes.borrow();
                    let mut output = output.lock().map_err(|_| {
                        Error::Playback("QQ Music audio output lock is poisoned".to_owned())
                    })?;
                    if retired.load(Ordering::SeqCst)
                        || response.is_closed()
                        || *current != generation
                    {
                        return Err(Error::Playback("QQ Music seek was cancelled".to_owned()));
                    }
                    let previous = output.as_ref().ok_or_else(|| {
                        Error::Playback("No QQ Music song has been loaded".to_owned())
                    })?;
                    let stream = stream
                        .as_ref()
                        .expect("a loaded track owns an output stream");
                    let next = Sink::connect_new(stream.mixer());
                    if previous.sink.is_paused() {
                        next.pause();
                    }
                    next.append(decoder);
                    let track = previous.track.clone();
                    previous.sink.stop();
                    *output = Some(Output {
                        sink: next,
                        offset: position,
                        track,
                    });
                    Ok(())
                })();
                let _ = response.send(result);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/qq_audio.rs"]
mod tests;
