use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

use crate::CommandResponse;

#[derive(Default)]
pub(crate) struct MusicState {
    spotify: tokio::sync::Mutex<Option<Arc<music::Spotify>>>,
    qq_music: tokio::sync::Mutex<Option<Arc<music::QqMusic>>>,
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

impl MusicState {
    async fn spotify(&self) -> Result<Arc<music::Spotify>, String> {
        let mut runtime = self.spotify.lock().await;
        if let Some(spotify) = runtime.as_ref() {
            return Ok(Arc::clone(spotify));
        }
        let credentials = match vesper_credentials::spotify().map_err(|error| error.to_string())? {
            vesper_credentials::Stored::Ready(credentials) => credentials,
            vesper_credentials::Stored::Missing => {
                return Err("Spotify is not connected. Open Settings to connect it.".to_owned());
            }
        };
        let spotify =
            Arc::new(music::Spotify::new(credentials).map_err(|error| error.to_string())?);
        *runtime = Some(Arc::clone(&spotify));
        Ok(spotify)
    }

    async fn qq_music(&self) -> Result<Arc<music::QqMusic>, String> {
        let mut runtime = self.qq_music.lock().await;
        if let Some(qq_music) = runtime.as_ref() {
            return Ok(Arc::clone(qq_music));
        }
        let credentials = match vesper_credentials::qq_music().map_err(|error| error.to_string())? {
            vesper_credentials::Stored::Ready(credentials) => credentials,
            vesper_credentials::Stored::Missing => {
                return Err("QQ Music is not connected. Connect it in Settings.".to_owned());
            }
        };
        let qq_music =
            Arc::new(music::QqMusic::new(credentials).map_err(|error| error.to_string())?);
        *runtime = Some(Arc::clone(&qq_music));
        Ok(qq_music)
    }

    pub(crate) async fn cover(&self, key: &str) -> Result<music::Cover, String> {
        if key.starts_with("spotify/") {
            return self
                .spotify()
                .await?
                .cover(key)
                .await
                .map_err(|error| error.to_string());
        }
        if key.starts_with("qq/") {
            return self
                .qq_music()
                .await?
                .cover(key)
                .await
                .map_err(|error| error.to_string());
        }
        Err("Unknown music cover provider".to_owned())
    }

    async fn pause_inactive(&self, provider: music::Provider) -> music::Result<()> {
        match provider {
            music::Provider::Spotify => {
                if let Some(qq_music) = self.qq_music.lock().await.as_ref() {
                    qq_music.pause().await?;
                }
            }
            music::Provider::QqMusic => {
                let spotify = self.spotify.lock().await.clone();
                if let Some(spotify) = spotify {
                    spotify.pause_if_playing().await;
                }
            }
        }
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn begin_qq_music_login(app: tauri::AppHandle) -> CommandResponse<music::QqQr> {
    let state = app.state::<MusicState>();
    let Some(_operation) = state.qq_login_operation.enter() else {
        return CommandResponse::Failed {
            message: "Another QQ Music login operation is already running".to_owned(),
        };
    };
    let generation = state.qq_login_generation.fetch_add(1, Ordering::AcqRel) + 1;
    match music::QqLogin::start().await {
        Ok((login, qr)) if state.qq_login_generation.load(Ordering::Acquire) == generation => {
            *state.qq_login.lock().await = Some(login);
            CommandResponse::Ready { data: qr }
        }
        Ok(_) => CommandResponse::Failed {
            message: "QQ Music login was cancelled".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn poll_qq_music_login(
    app: tauri::AppHandle,
) -> CommandResponse<music::QqLoginStatus> {
    let state = app.state::<MusicState>();
    let Some(_operation) = state.qq_login_operation.enter() else {
        return CommandResponse::Failed {
            message: "Another QQ Music login operation is already running".to_owned(),
        };
    };
    let generation = state.qq_login_generation.load(Ordering::Acquire);
    let Some(mut active) = state.qq_login.lock().await.take() else {
        return CommandResponse::Failed {
            message: "QQ Music login is not active".to_owned(),
        };
    };
    match active.poll().await {
        Ok((status, credentials)) => {
            let mut login = state.qq_login.lock().await;
            if state.qq_login_generation.load(Ordering::Acquire) != generation {
                return CommandResponse::Failed {
                    message: "QQ Music login was cancelled".to_owned(),
                };
            }
            if let Some(credentials) = credentials {
                let mut runtime = state.qq_music.lock().await;
                if let Some(previous) = runtime.take() {
                    previous.shutdown().await;
                }
                if let Err(error) = vesper_credentials::save_qq_music(credentials) {
                    return CommandResponse::Failed {
                        message: error.to_string(),
                    };
                }
            }
            if !matches!(
                status,
                music::QqLoginStatus::Complete | music::QqLoginStatus::Expired
            ) {
                *login = Some(active);
            }
            CommandResponse::Ready { data: status }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn cancel_qq_music_login(app: tauri::AppHandle) -> CommandResponse<()> {
    let state = app.state::<MusicState>();
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
    let state = app.state::<MusicState>();
    let Some(_operation) = state.spotify_authorization.enter() else {
        return CommandResponse::Failed {
            message: "Spotify authorization is already running".to_owned(),
        };
    };
    let client_id = client_id
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let authorization = match music::web_authorization(client_id.as_deref()).await {
        Ok(authorization) => authorization,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    if let Err(error) = app.opener().open_url(&authorization.url, None::<String>) {
        return CommandResponse::Failed {
            message: format!("Could not open Spotify sign-in: {error}"),
        };
    }
    let web_token = match music::authenticate(authorization).await {
        Ok(token) => token,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let authorization = match music::playback_authorization().await {
        Ok(authorization) => authorization,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    if let Err(error) = app.opener().open_url(&authorization.url, None::<String>) {
        return CommandResponse::Failed {
            message: format!("Could not open Spotify playback authorization: {error}"),
        };
    }
    let playback_token = match music::authenticate(authorization).await {
        Ok(token) => token,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let credentials = vesper_credentials::SpotifyCredentials {
        web_client_id: client_id,
        web_refresh_token: web_token.refresh_token,
        playback_refresh_token: playback_token.refresh_token,
    };
    let mut runtime = state.spotify.lock().await;
    if let Some(previous) = runtime.take() {
        previous.shutdown().await;
    }
    if let Err(error) = vesper_credentials::save_spotify(credentials) {
        return CommandResponse::Failed {
            message: error.to_string(),
        };
    }
    CommandResponse::Ready {
        data: "spotify".to_owned(),
    }
}

#[tauri::command]
pub(crate) async fn read_music_tracks(
    provider: music::Provider,
    app: tauri::AppHandle,
) -> CommandResponse<Vec<music::Track>> {
    let state = app.state::<MusicState>();
    let selection = state
        .playback_actions
        .run(async {
            match state.pause_inactive(provider).await {
                Ok(()) => CommandResponse::Ready { data: () },
                Err(error) => CommandResponse::Failed {
                    message: error.to_string(),
                },
            }
        })
        .await;
    if let CommandResponse::Failed { message } = selection {
        return CommandResponse::Failed { message };
    }
    let result = match provider {
        music::Provider::Spotify => match state.spotify().await {
            Ok(spotify) => spotify.liked_songs().await,
            Err(message) => return CommandResponse::Failed { message },
        },
        music::Provider::QqMusic => match state.qq_music().await {
            Ok(qq_music) => qq_music.daily_songs().await,
            Err(message) => return CommandResponse::Failed { message },
        },
    };
    match result {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn read_music_playback(
    provider: music::Provider,
    app: tauri::AppHandle,
) -> CommandResponse<Option<music::Playback>> {
    let state = app.state::<MusicState>();
    let result = match provider {
        music::Provider::Spotify => match state.spotify().await {
            Ok(spotify) => spotify.playback().await,
            Err(message) => return CommandResponse::Failed { message },
        },
        music::Provider::QqMusic => match state.qq_music().await {
            Ok(qq_music) => qq_music.playback().await,
            Err(message) => return CommandResponse::Failed { message },
        },
    };
    match result {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn play_music_track(
    provider: music::Provider,
    track_id: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<MusicState>();
    state
        .playback_actions
        .run(async {
            if let Err(error) = state.pause_inactive(provider).await {
                return CommandResponse::Failed {
                    message: error.to_string(),
                };
            }
            let result = match provider {
                music::Provider::Spotify => match state.spotify().await {
                    Ok(spotify) => spotify.play(&track_id).await,
                    Err(message) => return CommandResponse::Failed { message },
                },
                music::Provider::QqMusic => match state.qq_music().await {
                    Ok(qq_music) => qq_music.play(&track_id).await,
                    Err(message) => return CommandResponse::Failed { message },
                },
            };
            action_response(provider, result)
        })
        .await
}

#[tauri::command]
pub(crate) async fn resume_music(
    provider: music::Provider,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<MusicState>();
    state
        .playback_actions
        .run(async {
            if let Err(error) = state.pause_inactive(provider).await {
                return CommandResponse::Failed {
                    message: error.to_string(),
                };
            }
            let result = match provider {
                music::Provider::Spotify => match state.spotify().await {
                    Ok(spotify) => spotify.resume().await,
                    Err(message) => return CommandResponse::Failed { message },
                },
                music::Provider::QqMusic => match state.qq_music().await {
                    Ok(qq_music) => qq_music.resume().await,
                    Err(message) => return CommandResponse::Failed { message },
                },
            };
            action_response(provider, result)
        })
        .await
}

#[tauri::command]
pub(crate) async fn pause_music(
    provider: music::Provider,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<MusicState>();
    state
        .playback_actions
        .run(async {
            let result = match provider {
                music::Provider::Spotify => match state.spotify().await {
                    Ok(spotify) => spotify.pause().await,
                    Err(message) => return CommandResponse::Failed { message },
                },
                music::Provider::QqMusic => match state.qq_music().await {
                    Ok(qq_music) => qq_music.pause().await,
                    Err(message) => return CommandResponse::Failed { message },
                },
            };
            action_response(provider, result)
        })
        .await
}

#[tauri::command]
pub(crate) async fn seek_music(
    provider: music::Provider,
    position_ms: u64,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<MusicState>();
    state
        .playback_actions
        .run(async {
            let result = match provider {
                music::Provider::Spotify => match state.spotify().await {
                    Ok(spotify) => spotify.seek(position_ms).await,
                    Err(message) => return CommandResponse::Failed { message },
                },
                music::Provider::QqMusic => match state.qq_music().await {
                    Ok(qq_music) => qq_music.seek(position_ms).await,
                    Err(message) => return CommandResponse::Failed { message },
                },
            };
            action_response(provider, result)
        })
        .await
}

#[tauri::command]
pub(crate) async fn set_music_playback_order(
    provider: music::Provider,
    order: music::PlaybackOrder,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let state = app.state::<MusicState>();
    let result = match provider {
        music::Provider::Spotify => match state.spotify().await {
            Ok(spotify) => spotify.set_playback_order(order).await,
            Err(message) => return CommandResponse::Failed { message },
        },
        music::Provider::QqMusic => match state.qq_music().await {
            Ok(qq_music) => qq_music.set_playback_order(order).await,
            Err(message) => return CommandResponse::Failed { message },
        },
    };
    action_response(provider, result)
}

#[tauri::command]
pub(crate) async fn read_music_lyrics(
    provider: music::Provider,
    track_id: String,
    app: tauri::AppHandle,
) -> CommandResponse<Option<music::Lyrics>> {
    let state = app.state::<MusicState>();
    let result = match provider {
        music::Provider::Spotify => match state.spotify().await {
            Ok(spotify) => spotify.lyrics(&track_id).await,
            Err(message) => return CommandResponse::Failed { message },
        },
        music::Provider::QqMusic => match state.qq_music().await {
            Ok(qq_music) => qq_music.lyrics(&track_id).await,
            Err(message) => return CommandResponse::Failed { message },
        },
    };
    match result {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

fn action_response(
    provider: music::Provider,
    result: music::Result<()>,
) -> CommandResponse<String> {
    match result {
        Ok(()) => CommandResponse::Ready {
            data: match provider {
                music::Provider::Spotify => "spotify",
                music::Provider::QqMusic => "qqMusic",
            }
            .to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[cfg(test)]
#[path = "../tests/unit/music.rs"]
mod tests;
