use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

use crate::{CommandError, CommandResponse};
use music::{Provider, QqMusic, Spotify};
use vault::Stored;

#[derive(Default)]
pub(crate) struct Music {
    spotify: tokio::sync::Mutex<Option<Arc<Spotify>>>,
    qq_music: tokio::sync::Mutex<Option<Arc<QqMusic>>>,
    qq_login: tokio::sync::Mutex<Option<music::QqLogin>>,
    spotify_authorization: OperationGate,
    qq_login_operation: OperationGate,
    qq_login_generation: AtomicU64,
    playback_actions: PlaybackActions,
}

// Playback commands can spend time waiting for another provider or runtime initialization.
// Select the latest command before any of that work, and drop superseded work before
// the next command pauses the old provider and starts its own audio.
struct PlaybackActions {
    generation: tokio::sync::watch::Sender<u64>,
    action: tokio::sync::Mutex<()>,
}

impl Default for PlaybackActions {
    fn default() -> Self {
        Self {
            generation: tokio::sync::watch::channel(0).0,
            action: tokio::sync::Mutex::new(()),
        }
    }
}

impl PlaybackActions {
    async fn run<T>(
        &self,
        action: impl std::future::Future<Output = CommandResponse<T>>,
    ) -> CommandResponse<T> {
        let mut generation = self.generation.subscribe();
        let mut version = 0;
        self.generation.send_modify(|current| {
            *current += 1;
            version = *current;
        });
        tokio::select! {
            biased;
            _ = generation.wait_for(|current| *current != version) => CommandResponse::Failed {
                message: "Music playback command was superseded".to_owned(),
            },
            response = async {
                let _action = self.action.lock().await;
                action.await
            } => response,
        }
    }
}

#[derive(Default)]
struct OperationGate(AtomicBool);

impl OperationGate {
    fn enter(&self) -> Option<OperationLease<'_>> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| OperationLease { gate: self })
    }
}

struct OperationLease<'a> {
    gate: &'a OperationGate,
}

impl Drop for OperationLease<'_> {
    fn drop(&mut self) {
        self.gate.0.store(false, Ordering::Release);
    }
}

enum Player {
    Spotify(Arc<Spotify>),
    QqMusic(Arc<QqMusic>),
}

impl Player {
    async fn tracks(&self) -> music::Result<Vec<music::Track>> {
        match self {
            Self::Spotify(spotify) => spotify.liked_songs().await,
            Self::QqMusic(qq_music) => qq_music.daily_songs().await,
        }
    }

    async fn playback(&self) -> music::Result<Option<music::Playback>> {
        match self {
            Self::Spotify(spotify) => spotify.playback().await,
            Self::QqMusic(qq_music) => qq_music.playback().await,
        }
    }

    async fn play(&self, track_id: &str) -> music::Result<()> {
        match self {
            Self::Spotify(spotify) => spotify.play(track_id).await,
            Self::QqMusic(qq_music) => qq_music.play(track_id).await,
        }
    }

    async fn resume(&self) -> music::Result<()> {
        match self {
            Self::Spotify(spotify) => spotify.resume().await,
            Self::QqMusic(qq_music) => qq_music.resume().await,
        }
    }

    async fn pause(&self) -> music::Result<()> {
        match self {
            Self::Spotify(spotify) => spotify.pause().await,
            Self::QqMusic(qq_music) => qq_music.pause().await,
        }
    }

    async fn seek(&self, position_ms: u64) -> music::Result<()> {
        match self {
            Self::Spotify(spotify) => spotify.seek(position_ms).await,
            Self::QqMusic(qq_music) => qq_music.seek(position_ms).await,
        }
    }

    async fn set_playback_order(&self, order: music::PlaybackOrder) -> music::Result<()> {
        match self {
            Self::Spotify(spotify) => spotify.set_playback_order(order).await,
            Self::QqMusic(qq_music) => qq_music.set_playback_order(order).await,
        }
    }

    async fn lyrics(&self, track_id: &str) -> music::Result<Option<music::Lyrics>> {
        match self {
            Self::Spotify(spotify) => spotify.lyrics(track_id).await,
            Self::QqMusic(qq_music) => qq_music.lyrics(track_id).await,
        }
    }
}

impl Music {
    async fn spotify(&self) -> Result<Arc<Spotify>, String> {
        let mut runtime = self.spotify.lock().await;
        if let Some(spotify) = runtime.as_ref() {
            return Ok(Arc::clone(spotify));
        }
        let credentials =
            match music::spotify::credentials::read().map_err(|error| error.to_string())? {
                Stored::Ready(credentials) => credentials,
                Stored::Missing => {
                    return Err("Spotify is not connected. Open Settings to connect it.".to_owned());
                }
            };
        let spotify = Arc::new(Spotify::new(credentials).map_err(|error| error.to_string())?);
        *runtime = Some(Arc::clone(&spotify));
        Ok(spotify)
    }

    async fn qq_music(&self) -> Result<Arc<QqMusic>, String> {
        let mut runtime = self.qq_music.lock().await;
        if let Some(qq_music) = runtime.as_ref() {
            return Ok(Arc::clone(qq_music));
        }
        let credentials = match music::qq::credentials::read().map_err(|error| error.to_string())? {
            Stored::Ready(credentials) => credentials,
            Stored::Missing => {
                return Err("QQ Music is not connected. Connect it in Settings.".to_owned());
            }
        };
        let qq_music = Arc::new(QqMusic::new(credentials).map_err(|error| error.to_string())?);
        *runtime = Some(Arc::clone(&qq_music));
        Ok(qq_music)
    }

    async fn player(&self, provider: Provider) -> Result<Player, String> {
        Ok(match provider {
            Provider::Spotify => Player::Spotify(self.spotify().await?),
            Provider::QqMusic => Player::QqMusic(self.qq_music().await?),
        })
    }

    pub(crate) async fn cover(&self, key: &str) -> Result<music::Cover, String> {
        let cover = if key.starts_with("spotify/") {
            self.spotify().await?.cover(key).await
        } else if key.starts_with("qq/") {
            self.qq_music().await?.cover(key).await
        } else {
            return Err("Unknown music cover provider".to_owned());
        };
        cover.map_err(|error| error.to_string())
    }

    async fn pause_inactive(&self, provider: Provider) -> music::Result<()> {
        match provider {
            Provider::Spotify => {
                if let Some(qq_music) = self.qq_music.lock().await.as_ref() {
                    qq_music.pause().await?;
                }
            }
            Provider::QqMusic => {
                let spotify = self.spotify.lock().await.clone();
                if let Some(spotify) = spotify {
                    spotify.pause_if_playing().await;
                }
            }
        }
        Ok(())
    }

    async fn select(&self, provider: Provider) -> Result<Player, CommandError> {
        self.pause_inactive(provider).await?;
        Ok(self.player(provider).await?)
    }
}

fn provider_name(provider: Provider) -> String {
    match provider {
        Provider::Spotify => "spotify",
        Provider::QqMusic => "qqMusic",
    }
    .to_owned()
}

#[tauri::command]
pub(crate) async fn begin_qq_music_login(app: tauri::AppHandle) -> CommandResponse<music::QqQr> {
    let state = app.state::<Music>();
    let Some(_operation) = state.qq_login_operation.enter() else {
        return CommandResponse::Failed {
            message: "Another QQ Music login operation is already running".to_owned(),
        };
    };
    let generation = state.qq_login_generation.fetch_add(1, Ordering::AcqRel) + 1;
    let result = async {
        let (login, qr) = music::QqLogin::start().await?;
        if state.qq_login_generation.load(Ordering::Acquire) != generation {
            return Err("QQ Music login was cancelled".into());
        }
        *state.qq_login.lock().await = Some(login);
        Ok::<_, CommandError>(qr)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn poll_qq_music_login(
    app: tauri::AppHandle,
) -> CommandResponse<music::QqLoginStatus> {
    let state = app.state::<Music>();
    let Some(_operation) = state.qq_login_operation.enter() else {
        return CommandResponse::Failed {
            message: "Another QQ Music login operation is already running".to_owned(),
        };
    };
    let generation = state.qq_login_generation.load(Ordering::Acquire);
    let result = async {
        let mut active = state
            .qq_login
            .lock()
            .await
            .take()
            .ok_or("QQ Music login is not active")?;
        let (status, credentials) = active.poll().await?;
        let mut login = state.qq_login.lock().await;
        if state.qq_login_generation.load(Ordering::Acquire) != generation {
            return Err("QQ Music login was cancelled".into());
        }
        if let Some(credentials) = credentials {
            let mut runtime = state.qq_music.lock().await;
            if let Some(previous) = runtime.take() {
                previous.shutdown().await;
            }
            music::qq::credentials::save(&credentials)?;
        }
        if !matches!(
            status,
            music::QqLoginStatus::Complete | music::QqLoginStatus::Expired
        ) {
            *login = Some(active);
        }
        Ok::<_, CommandError>(status)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn cancel_qq_music_login(app: tauri::AppHandle) -> CommandResponse<()> {
    let state = app.state::<Music>();
    let mut login = state.qq_login.lock().await;
    state.qq_login_generation.fetch_add(1, Ordering::AcqRel);
    *login = None;
    CommandResponse::Ready { data: () }
}

#[tauri::command]
pub(crate) async fn connect_spotify(
    app: tauri::AppHandle,
    client_id: Option<String>,
) -> CommandResponse<String> {
    let state = app.state::<Music>();
    let Some(_operation) = state.spotify_authorization.enter() else {
        return CommandResponse::Failed {
            message: "Spotify authorization is already running".to_owned(),
        };
    };
    let client_id = client_id
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let result = async {
        let authorization = music::web_authorization(client_id.as_deref()).await?;
        app.opener()
            .open_url(&authorization.url, None::<String>)
            .map_err(|error| format!("Could not open Spotify sign-in: {error}"))?;
        let web_token = music::authenticate(authorization).await?;
        let authorization = music::playback_authorization().await?;
        app.opener()
            .open_url(&authorization.url, None::<String>)
            .map_err(|error| format!("Could not open Spotify playback authorization: {error}"))?;
        let playback_token = music::authenticate(authorization).await?;
        let credentials = music::spotify::credentials::Credentials {
            web_client_id: client_id,
            web_refresh_token: web_token.refresh_token,
            playback_refresh_token: playback_token.refresh_token,
        };
        let mut runtime = state.spotify.lock().await;
        if let Some(previous) = runtime.take() {
            previous.shutdown().await;
        }
        music::spotify::credentials::save(&credentials)?;
        Ok::<_, CommandError>("spotify".to_owned())
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn read_music_tracks(
    provider: Provider,
    app: tauri::AppHandle,
) -> CommandResponse<Vec<music::Track>> {
    let state = app.state::<Music>();
    let selection = state
        .playback_actions
        .run(async { CommandResponse::from(state.pause_inactive(provider).await) })
        .await;
    if let CommandResponse::Failed { message } = selection {
        return CommandResponse::Failed { message };
    }
    let result = async { Ok::<_, CommandError>(state.player(provider).await?.tracks().await?) };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn read_music_playback(
    provider: Provider,
    app: tauri::AppHandle,
) -> CommandResponse<Option<music::Playback>> {
    let state = app.state::<Music>();
    let result = async { Ok::<_, CommandError>(state.player(provider).await?.playback().await?) };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn play_music_track(
    provider: Provider,
    track_id: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<Music>();
    let action = async {
        state.select(provider).await?.play(&track_id).await?;
        Ok::<_, CommandError>(provider_name(provider))
    };
    state
        .playback_actions
        .run(async { action.await.into() })
        .await
}

#[tauri::command]
pub(crate) async fn resume_music(
    provider: Provider,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<Music>();
    let action = async {
        state.select(provider).await?.resume().await?;
        Ok::<_, CommandError>(provider_name(provider))
    };
    state
        .playback_actions
        .run(async { action.await.into() })
        .await
}

#[tauri::command]
pub(crate) async fn pause_music(
    provider: Provider,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<Music>();
    let action = async {
        state.player(provider).await?.pause().await?;
        Ok::<_, CommandError>(provider_name(provider))
    };
    state
        .playback_actions
        .run(async { action.await.into() })
        .await
}

#[tauri::command]
pub(crate) async fn seek_music(
    provider: Provider,
    position_ms: u64,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<Music>();
    let action = async {
        state.player(provider).await?.seek(position_ms).await?;
        Ok::<_, CommandError>(provider_name(provider))
    };
    state
        .playback_actions
        .run(async { action.await.into() })
        .await
}

#[tauri::command]
pub(crate) async fn set_music_playback_order(
    provider: Provider,
    order: music::PlaybackOrder,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<Music>();
    let result = async {
        state
            .player(provider)
            .await?
            .set_playback_order(order)
            .await?;
        Ok::<_, CommandError>(provider_name(provider))
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn read_music_lyrics(
    provider: Provider,
    track_id: String,
    app: tauri::AppHandle,
) -> CommandResponse<Option<music::Lyrics>> {
    let state = app.state::<Music>();
    let result =
        async { Ok::<_, CommandError>(state.player(provider).await?.lyrics(&track_id).await?) };
    result.await.into()
}

#[cfg(test)]
#[path = "../tests/unit/music.rs"]
mod tests;
