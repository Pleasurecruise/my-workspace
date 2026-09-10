use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Weak};
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
    generation: u64,
    pending_loads: VecDeque<u64>,
    request_id: Option<u64>,
    last_request_id: Option<u64>,
    autoplay: bool,
    ended: bool,
    closed: bool,
    failure: Option<String>,
}

impl State {
    // Loads and their request-ID events share FIFO ordering, including repeated loads of one song.
    fn select(&mut self, id: String, duration_ms: u64) {
        self.generation += 1;
        self.pending_loads.push_back(self.generation);
        self.request_id = None;
        self.track_id = Some(id);
        self.duration_ms = duration_ms;
        self.progress_ms = 0;
        self.playing = false;
        self.autoplay = true;
        self.ended = false;
        self.failure = None;
    }
}

#[derive(Default)]
struct Queue {
    ids: Vec<String>,
    durations: HashMap<String, u64>,
}

pub(crate) struct LocalPlayer {
    session: Session,
    player: Arc<Player>,
    state: Arc<RwLock<State>>,
    queue: Arc<RwLock<Queue>>,
    events: tokio::task::JoinHandle<()>,
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
        let player = Player::new(config, session.clone(), Box::new(NoOpVolume), move || {
            backend(None, AudioFormat::default())
        });
        let state = Arc::new(RwLock::new(State::default()));
        let queue = Arc::new(RwLock::new(Queue::default()));
        let events = tokio::spawn(read_events(
            player.get_player_event_channel(),
            Arc::downgrade(&player),
            Arc::clone(&state),
            Arc::clone(&queue),
        ));
        Ok(Self {
            session,
            player,
            state,
            queue,
            events,
        })
    }

    pub async fn play(&self, track: &Track, tracks: &[Track]) -> Result<()> {
        let uri = track_uri(&track.id)?;
        let mut state = self.state.write().await;
        if state.closed {
            return Err(Error::Playback("Spotify player was closed".to_owned()));
        }
        {
            let mut queue = self.queue.write().await;
            queue.ids = tracks.iter().map(|track| track.id.clone()).collect();
            queue.durations = tracks
                .iter()
                .map(|track| (track.id.clone(), track.duration_ms))
                .collect();
        }
        state.select(track.id.clone(), track.duration_ms);
        self.player.load(uri, true, 0);
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if state.closed {
            return Err(Error::Playback("Spotify player was closed".to_owned()));
        }
        let id = state
            .track_id
            .clone()
            .ok_or_else(|| Error::Playback("No Spotify song has been loaded".to_owned()))?;
        state.autoplay = true;
        if state.ended || state.failure.is_some() {
            let uri = track_uri(&id)?;
            let duration = state.duration_ms;
            state.select(id, duration);
            self.player.load(uri, true, 0);
        } else {
            self.player.play();
        }
        Ok(())
    }

    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        state.autoplay = false;
        state.playing = false;
        self.player.pause();
    }

    pub async fn seek(&self, position_ms: u64) {
        let mut state = self.state.write().await;
        let position_ms = position_ms.min(state.duration_ms).min(u32::MAX as u64);
        state.progress_ms = position_ms;
        self.player.seek(position_ms as u32);
    }

    pub async fn shutdown(&self) {
        let mut state = self.state.write().await;
        state.closed = true;
        state.autoplay = false;
        state.playing = false;
        self.events.abort();
        self.session.shutdown();
        self.player.stop();
    }

    pub async fn set_order(&self, order: PlaybackOrder) {
        self.state.write().await.order = order;
    }

    pub async fn playback(&self) -> Result<Playback> {
        let state = self.state.read().await;
        if let Some(failure) = &state.failure {
            return Err(Error::Playback(failure.clone()));
        }
        Ok(Playback {
            track_id: state.track_id.clone(),
            playing: state.playing,
            progress_ms: state.progress_ms,
            duration_ms: state.duration_ms,
            order: state.order,
        })
    }
}

impl Drop for LocalPlayer {
    fn drop(&mut self) {
        self.events.abort();
        self.session.shutdown();
        self.player.stop();
    }
}

fn track_uri(track_id: &str) -> Result<SpotifyUri> {
    SpotifyUri::from_uri(&format!("spotify:track:{track_id}"))
        .map_err(|error| Error::InvalidData(error.to_string()))
}

async fn read_events(
    mut events: tokio::sync::mpsc::UnboundedReceiver<PlayerEvent>,
    player: Weak<Player>,
    state: Arc<RwLock<State>>,
    queue: Arc<RwLock<Queue>>,
) {
    while let Some(event) = events.recv().await {
        let mut state = state.write().await;
        if state.closed {
            return;
        }
        if let PlayerEvent::PlayRequestIdChanged { play_request_id } = event {
            // Seeking during loading can announce the same request again.
            if state.last_request_id == Some(play_request_id) {
                continue;
            }
            state.last_request_id = Some(play_request_id);
            if state.pending_loads.pop_front() == Some(state.generation) {
                state.request_id = Some(play_request_id);
            }
            continue;
        }
        if state.request_id.is_none() || event.get_play_request_id() != state.request_id {
            continue;
        }
        match event {
            PlayerEvent::Playing { position_ms, .. } => {
                state.playing = state.autoplay;
                state.progress_ms = u64::from(position_ms);
            }
            PlayerEvent::PositionChanged { position_ms, .. }
            | PlayerEvent::PositionCorrection { position_ms, .. }
            | PlayerEvent::Seeked { position_ms, .. } => {
                state.progress_ms = u64::from(position_ms);
            }
            PlayerEvent::Paused { position_ms, .. } => {
                state.playing = false;
                state.progress_ms = u64::from(position_ms);
            }
            PlayerEvent::Stopped { .. } => state.playing = false,
            PlayerEvent::EndOfTrack { .. } => {
                state.playing = false;
                state.ended = true;
                if !state.autoplay {
                    continue;
                }
                let queue = queue.read().await;
                let current_id = state.track_id.clone();
                let current = current_id.as_ref();
                let next = match state.order {
                    PlaybackOrder::Sequential => current
                        .and_then(|current| queue.ids.iter().position(|id| id == current))
                        .and_then(|index| queue.ids.get(index + 1)),
                    PlaybackOrder::RepeatOne => current,
                    PlaybackOrder::Shuffle => queue
                        .ids
                        .iter()
                        .filter(|id| Some(*id) != current)
                        .choose(&mut rand::rng())
                        .or(current),
                };
                if let Some(id) = next {
                    match track_uri(id) {
                        Ok(uri) => {
                            let Some(player) = player.upgrade() else {
                                return;
                            };
                            let duration = queue.durations.get(id).copied().unwrap_or(0);
                            state.select(id.clone(), duration);
                            player.load(uri, true, 0);
                        }
                        Err(error) => {
                            state.failure =
                                Some(format!("Could not play the next Spotify track: {error}"))
                        }
                    }
                }
            }
            PlayerEvent::Unavailable { .. } | PlayerEvent::AudioKeyUnavailable { .. } => {
                state.playing = false;
                state.failure = Some("Spotify track is unavailable for local playback".to_owned());
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spotify_player.rs"]
mod tests;
