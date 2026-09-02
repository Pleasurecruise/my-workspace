use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use hyper_util::client::proxy::matcher::Matcher;
use librespot_core::SpotifyUri;
use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::NoOpVolume;
use librespot_playback::player::{Player, PlayerEvent};
use rand::seq::IteratorRandom;
use tokio::sync::RwLock;

use crate::{Error, Result};

use super::{Playback, PlaybackOrder, Track};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Default)]
struct State {
    track_id: Option<String>,
    playing: bool,
    progress_ms: u64,
    duration_ms: u64,
    order: PlaybackOrder,
}

#[derive(Default)]
struct Queue {
    ids: Vec<String>,
    durations: HashMap<String, u64>,
}

pub(crate) struct LocalPlayer {
    player: Arc<Player>,
    state: Arc<RwLock<State>>,
    queue: Arc<RwLock<Queue>>,
}

impl LocalPlayer {
    pub async fn connect(access_token: String) -> Result<Self> {
        let mut session_config = SessionConfig::default();
        let destination = http::Uri::from_static("https://ap.spotify.com");
        if let Some(proxy) = Matcher::from_system().intercept(&destination) {
            session_config.proxy =
                Some(
                    proxy.uri().to_string().parse().map_err(|error| {
                        Error::Playback(format!("invalid system proxy: {error}"))
                    })?,
                );
            tracing::info!("Spotify playback is using the system proxy");
        }
        let session = Session::new(session_config, None);
        tokio::time::timeout(
            CONNECT_TIMEOUT,
            session.connect(Credentials::with_access_token(access_token), false),
        )
        .await
        .map_err(|_| Error::Playback("Spotify playback connection timed out".to_owned()))?
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("Travel restriction") {
                    Error::Playback(
                        "Spotify rejected the playback region. Configure a system HTTP(S) proxy whose region matches the Spotify account, then try again"
                            .to_owned(),
                    )
                } else {
                    Error::Playback(message)
                }
            })?;
        let backend = audio_backend::find(None)
            .ok_or_else(|| Error::Playback("no audio output backend is available".to_owned()))?;
        let config = PlayerConfig {
            position_update_interval: Some(Duration::from_millis(500)),
            ..PlayerConfig::default()
        };
        let player = Player::new(config, session, Box::new(NoOpVolume), move || {
            backend(None, AudioFormat::default())
        });
        let state = Arc::new(RwLock::new(State::default()));
        let queue = Arc::new(RwLock::new(Queue::default()));
        tokio::spawn(read_events(
            player.get_player_event_channel(),
            Arc::clone(&player),
            Arc::clone(&state),
            Arc::clone(&queue),
        ));
        Ok(Self {
            player,
            state,
            queue,
        })
    }

    pub async fn play(&self, track: &Track, tracks: &[Track]) -> Result<()> {
        let uri = track_uri(&track.id)?;
        let order = self.state.read().await.order;
        {
            let mut queue = self.queue.write().await;
            queue.ids = tracks.iter().map(|track| track.id.clone()).collect();
            queue.durations = tracks
                .iter()
                .map(|track| (track.id.clone(), track.duration_ms))
                .collect();
        }
        *self.state.write().await = State {
            track_id: Some(track.id.clone()),
            playing: true,
            progress_ms: 0,
            duration_ms: track.duration_ms,
            order,
        };
        self.player.load(uri, true, 0);
        Ok(())
    }

    pub async fn resume(&self) {
        self.state.write().await.playing = true;
        self.player.play();
    }

    pub async fn pause(&self) {
        self.state.write().await.playing = false;
        self.player.pause();
    }

    pub async fn seek(&self, position_ms: u64) {
        let position_ms = position_ms.min(u32::MAX as u64);
        self.state.write().await.progress_ms = position_ms;
        self.player.seek(position_ms as u32);
    }

    pub async fn set_order(&self, order: PlaybackOrder) {
        self.state.write().await.order = order;
    }

    pub async fn playback(&self) -> Playback {
        let state = self.state.read().await;
        Playback {
            track_id: state.track_id.clone(),
            playing: state.playing,
            progress_ms: state.progress_ms,
            duration_ms: state.duration_ms,
            order: state.order,
        }
    }
}

fn track_uri(track_id: &str) -> Result<SpotifyUri> {
    SpotifyUri::from_uri(&format!("spotify:track:{track_id}"))
        .map_err(|error| Error::InvalidData(error.to_string()))
}

async fn read_events(
    mut events: tokio::sync::mpsc::UnboundedReceiver<PlayerEvent>,
    player: Arc<Player>,
    state: Arc<RwLock<State>>,
    queue: Arc<RwLock<Queue>>,
) {
    while let Some(event) = events.recv().await {
        match event {
            PlayerEvent::Playing { position_ms, .. }
            | PlayerEvent::PositionChanged { position_ms, .. }
            | PlayerEvent::PositionCorrection { position_ms, .. }
            | PlayerEvent::Seeked { position_ms, .. } => {
                let mut state = state.write().await;
                state.playing = true;
                state.progress_ms = u64::from(position_ms);
            }
            PlayerEvent::Paused { position_ms, .. } => {
                let mut state = state.write().await;
                state.playing = false;
                state.progress_ms = u64::from(position_ms);
            }
            PlayerEvent::Stopped { .. } => state.write().await.playing = false,
            PlayerEvent::EndOfTrack { .. } => {
                let (current, order) = {
                    let state = state.read().await;
                    (state.track_id.clone(), state.order)
                };
                let next = {
                    let queue = queue.read().await;
                    let id = match order {
                        PlaybackOrder::Sequential => current
                            .as_ref()
                            .and_then(|current| queue.ids.iter().position(|id| id == current))
                            .and_then(|index| queue.ids.get(index + 1)),
                        PlaybackOrder::RepeatOne => current.as_ref(),
                        PlaybackOrder::Shuffle => queue
                            .ids
                            .iter()
                            .filter(|id| Some(id.as_str()) != current.as_deref())
                            .choose(&mut rand::rng())
                            .or(current.as_ref()),
                    };
                    id.map(|id| (id.clone(), queue.durations.get(id).copied().unwrap_or(0)))
                };
                match next {
                    Some((id, duration_ms)) => match track_uri(&id) {
                        Ok(uri) => {
                            *state.write().await = State {
                                track_id: Some(id),
                                playing: true,
                                progress_ms: 0,
                                duration_ms,
                                order,
                            };
                            player.load(uri, true, 0);
                        }
                        Err(error) => tracing::warn!(%error, "could not advance the music queue"),
                    },
                    None => state.write().await.playing = false,
                }
            }
            PlayerEvent::Unavailable { track_id, .. } => {
                tracing::warn!(%track_id, "Spotify track is unavailable for local playback");
                state.write().await.playing = false;
            }
            _ => {}
        }
    }
}
