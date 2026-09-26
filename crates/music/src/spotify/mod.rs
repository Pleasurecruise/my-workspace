use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio::sync::{Mutex, RwLock};

use self::player::LocalPlayer;
use crate::{Cover, Error, Lyrics, Playback, PlaybackOrder, Result, Track, lyrics};

mod auth;
mod player;

pub use auth::{authenticate, playback_authorization, web_authorization};

const API: &str = "https://api.spotify.com/v1";
const LIBRARY_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;
const TOKEN_MARGIN: Duration = Duration::from_secs(60);
const MAX_RATE_LIMIT_RETRIES: u32 = 3;
const MAX_RETRY_WAIT: Duration = Duration::from_secs(30);
const DEFAULT_RETRY_WAIT: Duration = Duration::from_secs(1);
const LIBRARY_REFRESH_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
struct LibraryRefresh {
    cooldown: Option<LibraryCooldown>,
    started_at: Option<Instant>,
    tracks: Vec<Track>,
    covers: HashMap<String, String>,
    offset: u64,
}

struct LibraryCooldown {
    started_at: Instant,
    wait: Duration,
    quota_exhausted: bool,
}

impl LibraryCooldown {
    fn remaining(&self) -> Duration {
        self.wait.saturating_sub(self.started_at.elapsed())
    }

    fn error(&self) -> Error {
        let remaining = self.remaining();
        let retry_after_secs = remaining
            .as_secs()
            .saturating_add(u64::from(remaining.subsec_nanos() > 0));
        if self.quota_exhausted {
            Error::SpotifyQuotaExhausted { retry_after_secs }
        } else {
            Error::SpotifyRateLimited { retry_after_secs }
        }
    }
}

#[derive(serde::Deserialize)]
struct SpotifyApiFailure {
    error: SpotifyApiReason,
}

#[derive(serde::Deserialize)]
struct SpotifyApiReason {
    reason: Option<String>,
}

struct Token {
    value: String,
    expires_at: Instant,
}

#[derive(Default)]
struct LibraryCache {
    tracks: Vec<Track>,
    loaded_at: Option<Instant>,
}

impl LibraryCache {
    fn fresh_tracks(&self, now: Instant) -> Option<Vec<Track>> {
        self.loaded_at
            .filter(|loaded_at| now.saturating_duration_since(*loaded_at) < LIBRARY_CACHE_TTL)
            .map(|_| self.tracks.clone())
    }
}

pub struct Spotify {
    http: reqwest::Client,
    api: String,
    credentials: Mutex<vesper_credentials::SpotifyCredentials>,
    token: Mutex<Option<Token>>,
    player: Mutex<Option<Arc<LocalPlayer>>>,
    covers: RwLock<HashMap<String, String>>,
    tracks: RwLock<HashMap<String, Track>>,
    library: RwLock<LibraryCache>,
    library_refresh: Mutex<LibraryRefresh>,
    closed: AtomicBool,
    playback_generation: AtomicU64,
    playback_action: Mutex<()>,
}

impl Spotify {
    pub fn new(credentials: vesper_credentials::SpotifyCredentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("Vesper Music/0.1")
            .build()?;
        Ok(Self {
            http,
            api: API.to_owned(),
            credentials: Mutex::new(credentials),
            token: Mutex::new(None),
            player: Mutex::new(None),
            covers: RwLock::new(HashMap::new()),
            tracks: RwLock::new(HashMap::new()),
            library: RwLock::new(LibraryCache::default()),
            library_refresh: Mutex::new(LibraryRefresh::default()),
            closed: AtomicBool::new(false),
            playback_generation: AtomicU64::new(0),
            playback_action: Mutex::new(()),
        })
    }

    pub async fn shutdown(&self) {
        self.closed.store(true, Ordering::SeqCst);
        let mut player = self.player.lock().await;
        if let Some(player) = player.take() {
            player.shutdown().await;
        }
        let _credentials = self.credentials.lock().await;
    }

    pub async fn liked_songs(&self) -> Result<Vec<Track>> {
        if let Some(tracks) = self.library.read().await.fresh_tracks(Instant::now()) {
            return Ok(tracks);
        }

        let mut refresh = self.library_refresh.lock().await;
        if let Some(tracks) = self.library.read().await.fresh_tracks(Instant::now()) {
            return Ok(tracks);
        }

        if let Some(cooldown) = &refresh.cooldown
            && !cooldown.remaining().is_zero()
        {
            return Err(cooldown.error());
        }
        if refresh
            .started_at
            .is_none_or(|started_at| started_at.elapsed() >= LIBRARY_REFRESH_TTL)
        {
            refresh.started_at = Some(Instant::now());
            refresh.tracks.clear();
            refresh.covers.clear();
            refresh.offset = 0;
        }
        let mut retries = 0;
        loop {
            if self.closed.load(Ordering::SeqCst) {
                return Err(Error::Playback("Spotify player was closed".to_owned()));
            }
            let offset = refresh.offset;
            let access_token = self.access_token().await?;
            let response = self
                .http
                .get(format!("{}/me/tracks", self.api))
                .bearer_auth(access_token)
                .query(&[("limit", 50_u64), ("offset", offset)])
                .send()
                .await?;
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let wait = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or(DEFAULT_RETRY_WAIT)
                    .max(Duration::from_secs(1));
                let quota_exhausted =
                    response
                        .json::<SpotifyApiFailure>()
                        .await
                        .ok()
                        .is_some_and(|failure| {
                            failure.error.reason.as_deref() == Some("QUOTA_EXCEEDED")
                        });
                let cooldown = LibraryCooldown {
                    started_at: Instant::now(),
                    wait,
                    quota_exhausted,
                };
                let error = cooldown.error();
                refresh.cooldown = Some(cooldown);
                tracing::warn!(
                    offset,
                    retry_after_secs = wait.as_secs(),
                    quota_exhausted,
                    "Spotify liked songs rate limited"
                );
                if quota_exhausted || wait > MAX_RETRY_WAIT || retries >= MAX_RATE_LIMIT_RETRIES {
                    return Err(error);
                }
                retries += 1;
                tokio::time::sleep(wait).await;
                continue;
            }
            let response = check(response, "read liked songs")?;
            let page: SavedTrackPage = response.json().await?;
            if self.closed.load(Ordering::SeqCst) {
                return Err(Error::Playback("Spotify player was closed".to_owned()));
            }
            let count = page.items.len();
            for saved in page.items {
                if saved.track.id.is_empty() {
                    continue;
                }
                let cover = saved
                    .track
                    .album
                    .images
                    .first()
                    .map(|image| image.url.clone());
                if let Some(url) = cover {
                    refresh
                        .covers
                        .insert(format!("spotify/{}", saved.track.id), url);
                }
                let track = Track {
                    cover_key: (!saved.track.album.images.is_empty())
                        .then(|| format!("spotify/{}", saved.track.id)),
                    id: saved.track.id,
                    name: saved.track.name,
                    artists: saved
                        .track
                        .artists
                        .into_iter()
                        .map(|artist| artist.name)
                        .collect(),
                    album: saved.track.album.name,
                    duration_ms: saved.track.duration_ms,
                    added_at: saved.added_at,
                };
                refresh.tracks.push(track);
            }
            refresh.offset += count as u64;
            if count == 0 || refresh.offset >= page.total || page.next.is_none() {
                break;
            }
        }
        let tracks = std::mem::take(&mut refresh.tracks);
        refresh.started_at = None;
        refresh.cooldown = None;
        *self.covers.write().await = std::mem::take(&mut refresh.covers);
        *self.tracks.write().await = tracks
            .iter()
            .map(|track| (track.id.clone(), track.clone()))
            .collect();
        *self.library.write().await = LibraryCache {
            tracks: tracks.clone(),
            loaded_at: Some(Instant::now()),
        };
        Ok(tracks)
    }

    pub async fn playback(&self) -> Result<Option<Playback>> {
        let player = self.player.lock().await.clone();
        match player {
            Some(player) => Ok(Some(player.playback().await?)),
            None => Ok(None),
        }
    }

    pub async fn play(&self, track_id: &str) -> Result<()> {
        if track_id.is_empty() {
            return Err(Error::InvalidData("track id is empty".to_owned()));
        }
        let generation = self.playback_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let tracks = self.tracks.read().await;
        self.check_playback_request(generation)?;
        let track = tracks
            .get(track_id)
            .cloned()
            .ok_or_else(|| Error::InvalidData("unknown track id".to_owned()))?;
        drop(tracks);
        let library = self.library.read().await.tracks.clone();
        self.check_playback_request(generation)?;
        let player = self.local_player().await?;
        let _action = self.playback_action.lock().await;
        self.check_playback_request(generation)?;
        player.play(&track, &library).await
    }

    pub async fn resume(&self) -> Result<()> {
        let generation = self.playback_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let player = self.local_player().await?;
        let _action = self.playback_action.lock().await;
        self.check_playback_request(generation)?;
        player.resume().await
    }

    fn check_playback_request(&self, generation: u64) -> Result<()> {
        if self.closed.load(Ordering::SeqCst)
            || self.playback_generation.load(Ordering::SeqCst) != generation
        {
            return Err(Error::Playback(
                "Spotify playback request was cancelled".to_owned(),
            ));
        }
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        self.pause_if_playing().await;
        Ok(())
    }

    pub async fn pause_if_playing(&self) {
        // Cancel requests still looking up songs or initializing the player, then
        // serialize the pause with the final load/resume so it cannot restart afterward.
        self.playback_generation.fetch_add(1, Ordering::SeqCst);
        let _action = self.playback_action.lock().await;
        if let Some(player) = self.player.lock().await.clone() {
            player.pause().await;
        }
    }

    pub async fn seek(&self, position_ms: u64) -> Result<()> {
        self.local_player().await?.seek(position_ms).await;
        Ok(())
    }

    pub async fn set_playback_order(&self, order: PlaybackOrder) -> Result<()> {
        self.local_player().await?.set_order(order).await;
        Ok(())
    }

    pub async fn lyrics(&self, track_id: &str) -> Result<Option<Lyrics>> {
        let track = self
            .tracks
            .read()
            .await
            .get(track_id)
            .cloned()
            .ok_or_else(|| Error::InvalidData("unknown track id".to_owned()))?;
        lyrics::read(
            &self.http,
            track
                .artists
                .first()
                .map(String::as_str)
                .unwrap_or_default(),
            &track.name,
            &track.album,
            track.duration_ms,
        )
        .await
    }

    pub async fn cover(&self, key: &str) -> Result<Cover> {
        let url = self
            .covers
            .read()
            .await
            .get(key)
            .cloned()
            .ok_or_else(|| Error::InvalidData("unknown cover key".to_owned()))?;
        let url = trusted_cover_url(&url)?;
        let response = self.http.get(url).send().await?;
        let response = check(response, "read album cover")?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim)
            .filter(|value| matches!(*value, "image/jpeg" | "image/png" | "image/webp"))
            .ok_or_else(|| {
                Error::InvalidData("Spotify returned an unsupported cover format".to_owned())
            })?
            .to_owned();
        let mut body = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = body.next().await {
            let chunk = chunk?;
            if bytes.len().saturating_add(chunk.len()) > MAX_COVER_BYTES {
                return Err(Error::InvalidData(
                    "album cover exceeds the 10 MB limit".to_owned(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(Cover {
            bytes,
            content_type,
        })
    }

    async fn access_token(&self) -> Result<String> {
        let mut token = self.token.lock().await;
        if let Some(token) = token.as_ref()
            && token.expires_at > Instant::now() + TOKEN_MARGIN
        {
            return Ok(token.value.clone());
        }
        let mut credentials = self.credentials.lock().await;
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Playback("Spotify player was closed".to_owned()));
        }
        let refreshed = auth::refresh(
            credentials
                .web_client_id
                .as_deref()
                .unwrap_or(auth::WEB_CLIENT_ID),
            &credentials.web_refresh_token,
        )
        .await?;
        Self::rotate_refresh_token(
            &mut credentials,
            refreshed.refresh_token,
            |credentials, rotated| credentials.web_refresh_token = rotated,
        )?;
        drop(credentials);
        let value = refreshed.value;
        *token = Some(Token {
            value: value.clone(),
            expires_at: Instant::now() + Duration::from_secs(refreshed.expires_in),
        });
        Ok(value)
    }

    async fn local_player(&self) -> Result<Arc<LocalPlayer>> {
        let mut player = self.player.lock().await;
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Playback("Spotify player was closed".to_owned()));
        }
        if let Some(player) = player.as_ref() {
            return Ok(Arc::clone(player));
        }
        let mut credentials = self.credentials.lock().await;
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Playback("Spotify player was closed".to_owned()));
        }
        let refreshed = auth::refresh(
            auth::PLAYBACK_CLIENT_ID,
            &credentials.playback_refresh_token,
        )
        .await?;
        Self::rotate_refresh_token(
            &mut credentials,
            refreshed.refresh_token,
            |credentials, rotated| credentials.playback_refresh_token = rotated,
        )?;
        drop(credentials);
        let connected =
            Arc::new(LocalPlayer::connect(&self.http, &self.api, refreshed.value).await?);
        *player = Some(Arc::clone(&connected));
        Ok(connected)
    }

    // Spotify rotates refresh tokens on use. Persist the replacement before the credentials
    // lock is released so a restart resumes with the newest token.
    fn rotate_refresh_token(
        credentials: &mut vesper_credentials::SpotifyCredentials,
        rotated: Option<String>,
        apply: impl FnOnce(&mut vesper_credentials::SpotifyCredentials, String),
    ) -> Result<()> {
        if let Some(rotated) = rotated {
            let mut next_credentials = credentials.clone();
            apply(&mut next_credentials, rotated);
            vesper_credentials::save_spotify(next_credentials.clone())?;
            *credentials = next_credentials;
        }
        Ok(())
    }
}

fn trusted_cover_url(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| Error::InvalidData("Spotify returned an invalid cover URL".to_owned()))?;
    let trusted_host = url.host_str().is_some_and(|host| {
        host == "i.scdn.co" || host.ends_with(".scdn.co") || host.ends_with(".spotifycdn.com")
    });
    if url.scheme() != "https" || !trusted_host {
        return Err(Error::InvalidData(
            "Spotify returned an untrusted cover URL".to_owned(),
        ));
    }
    Ok(url)
}

fn check(response: reqwest::Response, operation: &'static str) -> Result<reqwest::Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(Error::Status {
            operation,
            status: response.status(),
        })
    }
}

#[derive(serde::Deserialize)]
struct SavedTrackPage {
    items: Vec<SavedTrack>,
    next: Option<String>,
    total: u64,
}

#[derive(serde::Deserialize)]
struct SavedTrack {
    added_at: String,
    track: TrackWire,
}

#[derive(serde::Deserialize)]
struct TrackWire {
    id: String,
    name: String,
    artists: Vec<ArtistWire>,
    album: AlbumWire,
    duration_ms: u64,
}

#[derive(serde::Deserialize)]
struct ArtistWire {
    name: String,
}

#[derive(serde::Deserialize)]
struct AlbumWire {
    name: String,
    images: Vec<ImageWire>,
}

#[derive(serde::Deserialize)]
struct ImageWire {
    url: String,
}

#[cfg(test)]
mod tests {
    use super::{Error, Spotify};

    async fn mock_library(
        responses: Vec<(u16, &'static str, String)>,
    ) -> (Spotify, tokio::task::JoinHandle<Vec<String>>) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, headers, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(&mut stream);
                let mut line = String::new();
                reader.read_line(&mut line).await.unwrap();
                requests.push(line.trim().to_owned());
                loop {
                    line.clear();
                    reader.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                }
                stream.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
            requests
        });
        let mut spotify = Spotify::new(vesper_credentials::SpotifyCredentials {
            web_client_id: None,
            web_refresh_token: "unused-test-token".to_owned(),
            playback_refresh_token: "unused-test-token".to_owned(),
        })
        .unwrap();
        spotify.api = format!("http://{address}");
        spotify.http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        *spotify.token.lock().await = Some(super::Token {
            value: "synthetic-access-token".to_owned(),
            expires_at: Instant::now() + Duration::from_secs(3600),
        });
        (spotify, server)
    }

    fn saved_page(id: &str, next: Option<&str>) -> String {
        serde_json::json!({
            "items": [{"added_at": "2026-01-01", "track": {
                "id": id, "name": id, "duration_ms": 180000,
                "artists": [{"name": "Artist"}], "album": {"name": "Album", "images": []}
            }}], "next": next, "total": 2
        })
        .to_string()
    }

    #[tokio::test]
    async fn enforces_library_cooldown() {
        let (spotify, server) = mock_library(vec![
            (200, "", saved_page("first", Some("next"))),
            (
                429,
                "Retry-After: 3600\r\n",
                "private provider body".to_owned(),
            ),
            (200, "", saved_page("second", None)),
        ])
        .await;
        *spotify.library.write().await = LibraryCache {
            tracks: vec![track()],
            loaded_at: Some(Instant::now() - LIBRARY_CACHE_TTL),
        };
        let (first, concurrent) = tokio::join!(spotify.liked_songs(), spotify.liked_songs());
        for result in [first, concurrent] {
            let error = result.unwrap_err();
            assert!(matches!(
                error,
                Error::SpotifyRateLimited {
                    retry_after_secs: 3600..=3601
                }
            ));
            assert!(!error.to_string().contains("private provider body"));
        }
        assert_eq!(spotify.library.read().await.tracks[0].id, "track");
        assert_eq!(spotify.library_refresh.lock().await.offset, 1);
        spotify
            .library_refresh
            .lock()
            .await
            .cooldown
            .as_mut()
            .unwrap()
            .started_at -= Duration::from_secs(3601);
        let tracks = spotify.liked_songs().await.unwrap();
        assert_eq!(
            tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert_eq!(spotify.liked_songs().await.unwrap().len(), 2);
        assert_eq!(
            server.await.unwrap(),
            [
                "GET /me/tracks?limit=50&offset=0 HTTP/1.1",
                "GET /me/tracks?limit=50&offset=1 HTTP/1.1",
                "GET /me/tracks?limit=50&offset=1 HTTP/1.1",
            ]
        );
    }

    #[tokio::test]
    async fn cancels_library_on_shutdown() {
        let (spotify, server) = mock_library(vec![(200, "", saved_page("old", None))]).await;
        let token = spotify.token.lock().await;
        let read = spotify.liked_songs();
        tokio::pin!(read);
        assert!(futures_util::poll!(&mut read).is_pending());
        spotify.shutdown().await;
        drop(token);
        assert!(matches!(read.await, Err(Error::Playback(_))));
        assert!(spotify.library.read().await.loaded_at.is_none());
        assert_eq!(server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn restarts_expired_pagination() {
        let (spotify, server) = mock_library(vec![(200, "", saved_page("fresh", None))]).await;
        {
            let mut refresh = spotify.library_refresh.lock().await;
            refresh.started_at = Some(Instant::now() - super::LIBRARY_REFRESH_TTL);
            refresh.tracks.push(track());
            refresh.offset = 50;
        }
        let tracks = spotify.liked_songs().await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, "fresh");
        assert_eq!(
            server.await.unwrap(),
            ["GET /me/tracks?limit=50&offset=0 HTTP/1.1"]
        );
    }

    #[tokio::test]
    async fn bounds_cooldown_retries() {
        let responses = (0..4)
            .map(|_| (429, "Retry-After: 1\r\n", "{}".to_owned()))
            .collect();
        let (spotify, server) = mock_library(responses).await;
        let started = Instant::now();
        assert!(matches!(
            spotify.liked_songs().await,
            Err(Error::SpotifyRateLimited { .. })
        ));
        assert!(started.elapsed() >= Duration::from_secs(3));
        assert!(matches!(
            spotify.liked_songs().await,
            Err(Error::SpotifyRateLimited { .. })
        ));
        assert_eq!(server.await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn stops_on_quota_exhaustion() {
        let (spotify, server) = mock_library(vec![(
            429,
            "Retry-After: 1\r\n",
            r#"{"error":{"reason":"QUOTA_EXCEEDED","message":"synthetic-secret"}}"#.to_owned(),
        )])
        .await;
        let error = spotify.liked_songs().await.unwrap_err();
        assert!(matches!(error, Error::SpotifyQuotaExhausted { .. }));
        assert!(!error.to_string().contains("synthetic-secret"));
        assert_eq!(server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn handles_invalid_retry_after() {
        for header in [
            "",
            "Retry-After: invalid\r\n",
            "Retry-After: 18446744073709551615\r\n",
        ] {
            let count = if header.contains("18446744073709551615") {
                1
            } else {
                4
            };
            let (spotify, server) =
                mock_library((0..count).map(|_| (429, header, "{}".to_owned())).collect()).await;
            assert!(matches!(
                spotify.liked_songs().await,
                Err(Error::SpotifyRateLimited { .. })
            ));
            assert!(matches!(
                spotify.liked_songs().await,
                Err(Error::SpotifyRateLimited { .. })
            ));
            assert_eq!(server.await.unwrap().len(), count);
        }
    }

    #[tokio::test]
    async fn cancels_stale_play() {
        let spotify = Spotify::new(vesper_credentials::SpotifyCredentials {
            web_client_id: None,
            web_refresh_token: "unused-test-token".to_owned(),
            playback_refresh_token: "unused-test-token".to_owned(),
        })
        .unwrap();
        let mut tracks = spotify.tracks.write().await;
        tracks.insert("track".to_owned(), track());
        let play = spotify.play("track");
        tokio::pin!(play);
        assert!(futures_util::poll!(&mut play).is_pending());
        spotify.pause_if_playing().await;
        drop(tracks);
        assert!(
            matches!(play.await, Err(Error::Playback(message)) if message.contains("cancelled"))
        );
        assert!(
            spotify.player.lock().await.is_none(),
            "cancelled playback must not initialize a player"
        );
    }

    use std::time::{Duration, Instant};

    use super::{
        LIBRARY_CACHE_TTL, LibraryCache, PlaybackOrder, SavedTrackPage, Track, trusted_cover_url,
    };

    fn track() -> Track {
        Track {
            id: "track".to_owned(),
            name: "Song".to_owned(),
            artists: vec!["Artist".to_owned()],
            album: "Album".to_owned(),
            duration_ms: 180_000,
            added_at: "2026-01-01".to_owned(),
            cover_key: Some("track".to_owned()),
        }
    }

    #[test]
    fn parses_liked_song_page() {
        let page: SavedTrackPage = serde_json::from_str(
            r#"{"items":[{"added_at":"2026-01-01","track":{"id":"track","name":"Song","duration_ms":180000,"artists":[{"name":"Artist"}],"album":{"name":"Album","images":[{"url":"https://i.scdn.co/image/cover"}]}}}],"next":null,"total":1}"#,
        )
        .unwrap();
        assert_eq!(page.items[0].track.name, "Song");
        assert_eq!(page.total, 1);
    }

    #[test]
    fn serializes_playback_order() {
        let repeat: PlaybackOrder = serde_json::from_str(r#""repeatOne""#).unwrap();
        assert_eq!(repeat, PlaybackOrder::RepeatOne);
        assert_eq!(
            serde_json::to_string(&PlaybackOrder::Shuffle).unwrap(),
            r#""shuffle""#
        );
    }

    #[test]
    fn expires_library_cache() {
        let now = Instant::now();
        let cache = LibraryCache {
            tracks: vec![track()],
            loaded_at: Some(now),
        };

        assert_eq!(cache.fresh_tracks(now).unwrap()[0].id, "track");
        assert!(
            cache
                .fresh_tracks(now + LIBRARY_CACHE_TTL - Duration::from_millis(1))
                .is_some()
        );
        assert!(cache.fresh_tracks(now + LIBRARY_CACHE_TTL).is_none());
    }

    #[test]
    fn rejects_untrusted_cover_urls() {
        assert!(trusted_cover_url("https://i.scdn.co/image/cover").is_ok());
        assert!(trusted_cover_url("http://i.scdn.co/image/cover").is_err());
        assert!(trusted_cover_url("https://example.com/cover").is_err());
    }
}
