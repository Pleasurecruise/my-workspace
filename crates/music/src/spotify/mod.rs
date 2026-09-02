use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::Method;
use tokio::sync::{Mutex, RwLock};

use self::player::LocalPlayer;
use crate::{Error, Lyrics, Result, lyrics};

mod auth;
mod player;

pub use auth::{authenticate, playback_authorization, web_authorization};

const API: &str = "https://api.spotify.com/v1";
const LIBRARY_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;
const TOKEN_MARGIN: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: u64,
    pub added_at: String,
    pub cover_key: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    pub track_id: Option<String>,
    pub playing: bool,
    pub progress_ms: u64,
    pub duration_ms: u64,
    pub order: PlaybackOrder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackOrder {
    #[default]
    Sequential,
    RepeatOne,
    Shuffle,
}

pub struct Cover {
    pub bytes: Vec<u8>,
    pub content_type: String,
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
    credentials: Mutex<vesper_credentials::SpotifyCredentials>,
    token: Mutex<Option<Token>>,
    player: Mutex<Option<Arc<LocalPlayer>>>,
    covers: RwLock<HashMap<String, String>>,
    tracks: RwLock<HashMap<String, Track>>,
    library: RwLock<LibraryCache>,
    library_refresh: Mutex<()>,
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
            credentials: Mutex::new(credentials),
            token: Mutex::new(None),
            player: Mutex::new(None),
            covers: RwLock::new(HashMap::new()),
            tracks: RwLock::new(HashMap::new()),
            library: RwLock::new(LibraryCache::default()),
            library_refresh: Mutex::new(()),
        })
    }

    pub async fn liked_songs(&self) -> Result<Vec<Track>> {
        if let Some(tracks) = self.library.read().await.fresh_tracks(Instant::now()) {
            return Ok(tracks);
        }

        let _refresh = self.library_refresh.lock().await;
        if let Some(tracks) = self.library.read().await.fresh_tracks(Instant::now()) {
            return Ok(tracks);
        }

        let mut tracks = Vec::new();
        let mut covers = HashMap::new();
        let mut offset = 0_u64;
        loop {
            let response = self
                .authorized(Method::GET, "/me/tracks")
                .await?
                .query(&[("limit", 50_u64), ("offset", offset)])
                .send()
                .await?;
            let response = check(response, "read liked songs")?;
            let page: SavedTrackPage = response.json().await?;
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
                    covers.insert(format!("spotify/{}", saved.track.id), url);
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
                tracks.push(track);
            }
            offset += count as u64;
            if count == 0 || offset >= page.total || page.next.is_none() {
                break;
            }
        }
        *self.covers.write().await = covers;
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
            Some(player) => Ok(Some(player.playback().await)),
            None => Ok(None),
        }
    }

    pub async fn play(&self, track_id: &str) -> Result<()> {
        if track_id.is_empty() {
            return Err(Error::InvalidData("track id is empty".to_owned()));
        }
        let track = self
            .tracks
            .read()
            .await
            .get(track_id)
            .cloned()
            .ok_or_else(|| Error::InvalidData("unknown track id".to_owned()))?;
        let library = self.library.read().await.tracks.clone();
        self.local_player().await?.play(&track, &library).await
    }

    pub async fn resume(&self) -> Result<()> {
        self.local_player().await?.resume().await;
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        self.local_player().await?.pause().await;
        Ok(())
    }

    pub async fn pause_if_playing(&self) {
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

    async fn authorized(&self, method: Method, path: &str) -> Result<reqwest::RequestBuilder> {
        let access_token = self.access_token().await?;
        Ok(self
            .http
            .request(method, format!("{API}{path}"))
            .bearer_auth(access_token))
    }

    async fn access_token(&self) -> Result<String> {
        let mut token = self.token.lock().await;
        if let Some(token) = token.as_ref()
            && token.expires_at > Instant::now() + TOKEN_MARGIN
        {
            return Ok(token.value.clone());
        }
        let mut credentials = self.credentials.lock().await;
        let refreshed = auth::refresh(auth::WEB_CLIENT_ID, &credentials.web_refresh_token).await?;
        if let Some(next_refresh_token) = refreshed.refresh_token {
            let mut next_credentials = credentials.clone();
            next_credentials.web_refresh_token = next_refresh_token;
            vesper_credentials::save_spotify(next_credentials.clone())?;
            *credentials = next_credentials;
        }
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
        if let Some(player) = player.as_ref() {
            return Ok(Arc::clone(player));
        }
        let mut credentials = self.credentials.lock().await;
        let refreshed = auth::refresh(
            auth::PLAYBACK_CLIENT_ID,
            &credentials.playback_refresh_token,
        )
        .await?;
        if let Some(next_refresh_token) = refreshed.refresh_token {
            let mut next_credentials = credentials.clone();
            next_credentials.playback_refresh_token = next_refresh_token;
            vesper_credentials::save_spotify(next_credentials.clone())?;
            *credentials = next_credentials;
        }
        drop(credentials);
        let connected = Arc::new(LocalPlayer::connect(refreshed.value).await?);
        *player = Some(Arc::clone(&connected));
        Ok(connected)
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
    fn playback_order_uses_the_command_wire_names() {
        let repeat: PlaybackOrder = serde_json::from_str(r#""repeatOne""#).unwrap();
        assert_eq!(repeat, PlaybackOrder::RepeatOne);
        assert_eq!(
            serde_json::to_string(&PlaybackOrder::Shuffle).unwrap(),
            r#""shuffle""#
        );
    }

    #[test]
    fn library_cache_expires_after_five_minutes() {
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
