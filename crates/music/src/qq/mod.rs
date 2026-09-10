use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use futures_util::StreamExt;
use rand::Rng;
use tokio::sync::{Mutex, RwLock, watch};

use crate::lyrics;
use crate::spotify::{Cover, Playback, PlaybackOrder, Track};
use crate::{Error, Lyrics, Result};

mod audio;
mod qr;

use audio::AudioPlayer;

pub use qr::{QqLogin, QqLoginStatus, QqQr};

const API: &str = "https://u.y.qq.com/cgi-bin/musicu.fcg";
const RENEW_AFTER: Duration = Duration::from_secs(20 * 60 * 60);
const RENEW_RETRY: Duration = Duration::from_secs(60 * 60);
const REFERER: &str = "https://y.qq.com/";
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_MEDIA_BYTES: usize = 100 * 1024 * 1024;
const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;
const DAILY_TRACK_LIMIT: usize = 30;

pub struct QqMusic {
    http: reqwest::Client,
    credentials: Mutex<CredentialState>,
    tracks: RwLock<HashMap<String, QqTrack>>,
    covers: RwLock<HashMap<String, String>>,
    library: RwLock<Library>,
    library_refresh: Mutex<()>,
    playback: RwLock<PlaybackState>,
    audio: AudioPlayer,
    generation: watch::Sender<u64>,
    playback_operation: Mutex<()>,
    closed: AtomicBool,
}

#[derive(Clone)]
struct QqTrack {
    track: Track,
    media_mid: String,
}

struct QqSession {
    cookie: String,
    auth: QqAuth,
}

struct CredentialState {
    value: vesper_credentials::QqMusicCredentials,
    renewal_attempt: Option<Instant>,
}

#[derive(Default)]
struct Library {
    tracks: Vec<Track>,
    loaded_at: Option<Instant>,
}

impl Library {
    fn fresh(&self) -> Option<Vec<Track>> {
        self.loaded_at
            .filter(|loaded| loaded.elapsed() < CACHE_TTL)
            .map(|_| self.tracks.clone())
    }
}

#[derive(Default)]
struct PlaybackState {
    failure: Option<String>,
    // Retain the last loaded identity for a retry after an audio worker is retired.
    track_id: Option<String>,
    duration_ms: u64,
    order: PlaybackOrder,
}

impl QqMusic {
    pub fn new(credentials: vesper_credentials::QqMusicCredentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36 Vesper/0.1")
            .build()?;
        Ok(Self {
            http,
            credentials: Mutex::new(CredentialState {
                value: credentials,
                renewal_attempt: None,
            }),
            tracks: RwLock::new(HashMap::new()),
            covers: RwLock::new(HashMap::new()),
            library: RwLock::new(Library::default()),
            library_refresh: Mutex::new(()),
            playback: RwLock::new(PlaybackState::default()),
            audio: AudioPlayer::default(),
            generation: watch::channel(0).0,
            playback_operation: Mutex::new(()),
            closed: AtomicBool::new(false),
        })
    }

    pub async fn daily_songs(&self) -> Result<Vec<Track>> {
        if let Some(tracks) = self.library.read().await.fresh() {
            return Ok(tracks);
        }
        let _refresh = self.library_refresh.lock().await;
        if let Some(tracks) = self.library.read().await.fresh() {
            return Ok(tracks);
        }

        let session = self.session().await?;
        let auth = &session.auth;
        let response = self
            .http
            .post(API)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::COOKIE, &session.cookie)
            .json(&serde_json::json!({
                "comm": &auth,
                "feed": {
                    "module": "music.recommend.RecommendFeed",
                    "method": "get_recommend_feed",
                    "param": { "direction": 0, "page": 1, "s_num": 0, "v_cache": [] }
                }
            }))
            .send()
            .await?;
        let feed: FeedData =
            QqResponse::read(response, "read QQ Music recommendation feed").await?;
        let playlist_id = feed
            .shelves
            .into_iter()
            .flat_map(|shelf| shelf.niches)
            .flat_map(|niche| niche.cards)
            .find(|card| card.title == "每日30首" && !card.id.is_empty())
            .ok_or_else(|| {
                Error::InvalidData("QQ Music did not return the Daily 30 card".to_owned())
            })?
            .id
            .parse::<u64>()
            .map_err(|_| {
                Error::InvalidData("QQ Music returned an invalid Daily 30 playlist ID".to_owned())
            })?;
        let response = self
            .http
            .post(API)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::COOKIE, &session.cookie)
            .json(&serde_json::json!({
                "comm": &auth,
                "daily": {
                    "module": "music.srfDissInfo.aiDissInfo",
                    "method": "uniform_get_Dissinfo",
                    "param": {
                        "disstid": playlist_id,
                        "userinfo": 1,
                        "tag": 1,
                        "orderlist": 1,
                        "song_begin": 0,
                        "song_num": DAILY_TRACK_LIMIT,
                        "onlysonglist": 0,
                        "enc_host_uin": ""
                    }
                }
            }))
            .send()
            .await?;
        let daily: DailyData =
            QqResponse::read(response, "read QQ Music Daily 30 playlist").await?;
        let wires = daily.songs;

        let mut mapped = Vec::new();
        for wire in wires.into_iter().take(DAILY_TRACK_LIMIT) {
            if wire.name.is_empty() {
                continue;
            }
            let album_mid = wire.album.mid.clone();
            let cover_key = (!album_mid.is_empty()).then(|| format!("qq/{}", wire.mid));
            if let Some(key) = cover_key.as_ref() {
                self.covers.write().await.insert(
                    key.clone(),
                    format!("https://y.qq.com/music/photo_new/T002R300x300M000{album_mid}.jpg?max_age=2592000"),
                );
            }
            let track = Track {
                id: wire.mid.clone(),
                name: wire.name,
                artists: wire
                    .singers
                    .into_iter()
                    .map(|artist| artist.name)
                    .filter(|name| !name.is_empty())
                    .collect(),
                album: wire.album.name,
                duration_ms: wire.interval.saturating_mul(1_000),
                added_at: String::new(),
                cover_key,
            };
            mapped.push(QqTrack {
                track,
                media_mid: if wire.file.media_mid.is_empty() {
                    wire.mid
                } else {
                    wire.file.media_mid
                },
            });
        }
        if mapped.is_empty() {
            return Err(Error::InvalidData(
                "QQ Music returned no Daily 30 songs. Try again later or reconnect in Settings."
                    .to_owned(),
            ));
        }
        let tracks: Vec<Track> = mapped.iter().map(|item| item.track.clone()).collect();
        *self.tracks.write().await = mapped
            .into_iter()
            .map(|item| (item.track.id.clone(), item))
            .collect();
        *self.library.write().await = Library {
            tracks: tracks.clone(),
            loaded_at: Some(Instant::now()),
        };
        Ok(tracks)
    }

    pub async fn play(self: &Arc<Self>, track_id: &str) -> Result<()> {
        let generation = self.invalidate_playback();
        let result = self.load_track(track_id, generation).await;
        if result.is_err()
            && *self.generation.borrow() == generation
            && self.audio.snapshot().is_ok_and(|audio| !audio.ended)
        {
            self.spawn_completion_monitor(generation);
        }
        result
    }

    fn invalidate_playback(&self) -> u64 {
        let mut next = 0;
        self.generation.send_modify(|generation| {
            *generation += 1;
            next = *generation;
        });
        next
    }

    pub async fn shutdown(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.invalidate_playback();
        self.audio.retire(None);
        // Complete any old credential write before the replacement login is saved.
        let _credentials = self.credentials.lock().await;
    }

    pub async fn playback(&self) -> Result<Option<Playback>> {
        let state = self.playback.read().await;
        if let Some(failure) = &state.failure {
            return Err(Error::Playback(failure.clone()));
        }
        let audio = self.audio.snapshot()?;
        let track_id = audio
            .track
            .as_ref()
            .map(|track| track.id.clone())
            .or_else(|| state.track_id.clone());
        let Some(track_id) = track_id else {
            return Ok(None);
        };
        Ok(Some(Playback {
            track_id: Some(track_id),
            playing: audio.playing,
            progress_ms: audio.progress_ms,
            duration_ms: audio
                .track
                .as_ref()
                .map_or(state.duration_ms, |track| track.duration_ms),
            order: state.order,
        }))
    }

    pub async fn resume(self: &Arc<Self>) -> Result<()> {
        let audio = self.audio.snapshot()?;
        let track_id = audio
            .track
            .as_ref()
            .map(|track| track.id.clone())
            .or(self.playback.read().await.track_id.clone())
            .ok_or_else(|| Error::Playback("No QQ Music song has been loaded".to_owned()))?;
        if audio.ended {
            return self.play(&track_id).await;
        }
        let generation = self.invalidate_playback();
        let _operation = self.playback_operation.lock().await;
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Playback("QQ Music player was closed".to_owned()));
        }
        {
            let current = self.generation.borrow();
            if *current != generation {
                return Err(Error::Playback(
                    "QQ Music playback was cancelled".to_owned(),
                ));
            }
            self.audio.resume()?;
        }
        self.playback.write().await.failure = None;
        self.spawn_completion_monitor(generation);
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        let generation = self.invalidate_playback();
        let current = self.generation.borrow();
        if *current != generation {
            return Err(Error::Playback("QQ Music pause was cancelled".to_owned()));
        }
        self.audio.pause()
    }

    pub async fn seek(self: &Arc<Self>, position_ms: u64) -> Result<()> {
        let generation = self.invalidate_playback();
        let mut changes = self.generation.subscribe();
        let result = tokio::select! {
            biased;
            _ = changes.wait_for(|current| *current != generation) => {
                Err(Error::Playback("QQ Music seek was cancelled".to_owned()))
            }
            result = async {
                let _operation = self.playback_operation.lock().await;
                if self.closed.load(Ordering::SeqCst) {
                    return Err(Error::Playback("QQ Music player was closed".to_owned()));
                }
                self.audio.seek(Duration::from_millis(position_ms), generation, self.generation.subscribe()).await
            } => result,
        };
        if *self.generation.borrow() == generation
            && (result.is_ok() || self.audio.snapshot().is_ok_and(|audio| !audio.ended))
        {
            self.spawn_completion_monitor(generation);
        }
        result
    }

    pub async fn set_playback_order(&self, order: PlaybackOrder) -> Result<()> {
        self.playback.write().await.order = order;
        Ok(())
    }

    pub async fn lyrics(&self, track_id: &str) -> Result<Option<Lyrics>> {
        if !self.tracks.read().await.contains_key(track_id) {
            return Err(Error::InvalidData("unknown QQ Music track id".to_owned()));
        }
        let session = self.session().await?;
        let response = self
            .http
            .post(API)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::COOKIE, &session.cookie)
            .json(&serde_json::json!({
                "comm": { "ct": 24, "cv": 0 },
                "lyric": {
                    "module": "music.musichallSong.PlayLyricInfo",
                    "method": "GetPlayLyricInfo",
                    "param": { "songMID": track_id }
                }
            }))
            .send()
            .await?;
        let data: Option<LyricData> = QqResponse::read(response, "read QQ Music lyrics").await?;
        let Some(encoded) = data.and_then(|data| data.lyric) else {
            return Ok(None);
        };
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or(encoded);
        Ok(lyrics::from_lrc(&decoded))
    }

    pub async fn cover(&self, key: &str) -> Result<Cover> {
        let url = self
            .covers
            .read()
            .await
            .get(key)
            .cloned()
            .ok_or_else(|| Error::InvalidData("unknown QQ Music cover key".to_owned()))?;
        let response = self
            .http
            .get(url)
            .header(reqwest::header::REFERER, REFERER)
            .send()
            .await?;
        read_bytes(response, "read QQ Music album cover", MAX_COVER_BYTES)
            .await
            .map(|bytes| Cover {
                bytes,
                content_type: "image/jpeg".to_owned(),
            })
    }

    async fn session(&self) -> Result<QqSession> {
        let mut credentials = self.credentials.lock().await;
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Playback("QQ Music player was closed".to_owned()));
        }
        let can_retry = credentials
            .renewal_attempt
            .is_none_or(|attempt| attempt.elapsed() >= RENEW_RETRY);
        if renewal_due(&credentials.value.cookie) && can_retry {
            credentials.renewal_attempt = Some(Instant::now());
            match self.renew(&credentials.value.cookie).await {
                Ok(cookie) => {
                    let renewed = vesper_credentials::QqMusicCredentials { cookie };
                    if let Err(error) = vesper_credentials::save_qq_music(renewed.clone()) {
                        tracing::warn!(%error, "could not store renewed QQ Music session");
                    }
                    credentials.value = renewed;
                }
                Err(error) => tracing::warn!(%error, "could not renew QQ Music session"),
            }
        }
        let cookie = credentials.value.cookie.clone();
        Ok(QqSession {
            auth: Self::auth(&cookie),
            cookie,
        })
    }

    async fn renew(&self, cookie: &str) -> Result<String> {
        let mut fields = parse_cookie(cookie);
        let key = fields
            .get("qm_keyst")
            .or_else(|| fields.get("qqmusic_key"))
            .cloned()
            .unwrap_or_default();
        let uin = fields
            .get("qm_str_musicid")
            .or_else(|| fields.get("uin"))
            .or_else(|| fields.get("wxuin"))
            .map(|value| value.trim_start_matches('o'))
            .unwrap_or("0");
        let login_type = fields
            .get("tmeLoginType")
            .or_else(|| fields.get("login_type"))
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or_else(|| if key.starts_with("W_X") { 1 } else { 2 });
        let param = if login_type == 1 {
            serde_json::json!({
                "openid": fields.get("wxopenid").or_else(|| fields.get("psrf_qqopenid")).map(String::as_str).unwrap_or_default(),
                "refresh_token": fields.get("wxrefresh_token").or_else(|| fields.get("psrf_qqrefresh_token")).map(String::as_str).unwrap_or_default(),
                "str_musicid": uin,
                "musickey": &key,
                "unionid": fields.get("psrf_qqunionid").map(String::as_str).unwrap_or_default(),
                "refresh_key": fields.get("qm_refresh_key").map(String::as_str).unwrap_or_default(),
                "loginMode": 2
            })
        } else {
            serde_json::json!({
                "openid": fields.get("psrf_qqopenid").or_else(|| fields.get("wxopenid")).map(String::as_str).unwrap_or_default(),
                "access_token": fields.get("psrf_qqaccess_token").map(String::as_str).unwrap_or_default(),
                "refresh_token": fields.get("psrf_qqrefresh_token").or_else(|| fields.get("wxrefresh_token")).map(String::as_str).unwrap_or_default(),
                "expired_in": fields.get("psrf_access_token_expiresAt").and_then(|value| value.parse::<u64>().ok()).unwrap_or_default(),
                "musicid": uin.parse::<u64>().unwrap_or_default(),
                "musickey": &key,
                "refresh_key": fields.get("qm_refresh_key").map(String::as_str).unwrap_or_default(),
                "loginMode": 2
            })
        };
        let response = self.http.post(API)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::COOKIE, cookie)
            .json(&serde_json::json!({
                "comm": {
                    "ct": 11, "cv": 14090008, "v": 14090008, "chid": "10003505",
                    "os_ver": "15", "phonetype": "24122RKC7C", "tmeAppID": "qqmusic",
                    "nettype": "NETWORK_WIFI", "udid": "0", "OpenUDID": "0", "QIMEI36": "0",
                    "uin": uin, "qq": uin, "authst": &key, "tmeLoginType": login_type
                },
                "request": { "module": "music.login.LoginServer", "method": "Login", "param": param }
            }))
            .send().await?;
        let data: RenewData = QqResponse::read(response, "renew QQ Music session").await?;
        if data.musickey.is_empty() {
            return Err(Error::Authentication(
                "QQ Music rejected session renewal".to_owned(),
            ));
        }
        data.apply(&mut fields, login_type);
        Ok(render_cookie(fields))
    }

    fn auth(cookie: &str) -> QqAuth {
        let cookies: HashMap<&str, &str> = cookie
            .split(';')
            .filter_map(|part| part.trim().split_once('='))
            .map(|(key, value)| (key.trim(), value.trim()))
            .collect();
        let login_type = cookies
            .get("tmeLoginType")
            .or_else(|| cookies.get("login_type"));
        let identity_fields = if login_type == Some(&"1") {
            ["wxuin", "uin", "qqmusic_uin", "p_uin"]
        } else {
            ["uin", "qqmusic_uin", "wxuin", "p_uin"]
        };
        let uin = identity_fields
            .iter()
            .find_map(|key| cookies.get(key).copied())
            .unwrap_or_default()
            .trim_start_matches('o')
            .trim_start_matches('0');
        let authst = ["qm_keyst", "qqmusic_key", "music_key", "wxskey"]
            .iter()
            .find_map(|key| cookies.get(key).copied())
            .unwrap_or_default();
        QqAuth {
            uin: if uin.is_empty() { "0" } else { uin }.to_owned(),
            format: "json",
            ct: 19,
            cv: 0,
            authst: authst.to_owned(),
            login_type: login_type
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or_else(|| if authst.starts_with("W_X") { 1 } else { 2 }),
            gtk: authst.bytes().fold(5_381_u32, |hash, byte| {
                hash.wrapping_add(hash.wrapping_shl(5))
                    .wrapping_add(u32::from(byte))
            }) & 0x7fff_ffff,
        }
    }

    async fn load_track(self: &Arc<Self>, track_id: &str, generation: u64) -> Result<()> {
        let mut changes = self.generation.subscribe();
        tokio::select! {
            biased;
            _ = changes.wait_for(|current| *current != generation) => {
                Err(Error::Playback("QQ Music playback was cancelled".to_owned()))
            }
            result = async {
                let _operation = self.playback_operation.lock().await;
                if self.closed.load(Ordering::SeqCst) {
                    return Err(Error::Playback("QQ Music player was closed".to_owned()));
                }
                let track = self
                    .tracks
                    .read()
                    .await
                    .get(track_id)
                    .cloned()
                    .ok_or_else(|| Error::InvalidData("unknown QQ Music track id".to_owned()))?;
                let session = self.session().await?;
                let url = self.play_url(&track, &session).await?;
                let response = self
                    .http
                    .get(url)
                    .header(reqwest::header::REFERER, REFERER)
                    .header(reqwest::header::COOKIE, &session.cookie)
                    .send()
                    .await?;
                let bytes = read_bytes(response, "download QQ Music audio", MAX_MEDIA_BYTES).await?;
                self.audio.load(bytes, track.track.clone(), generation, self.generation.subscribe()).await?;
                {
                    let mut state = self.playback.write().await;
                    let current = self.generation.borrow();
                    if *current != generation {
                        return Err(Error::Playback("QQ Music playback was cancelled".to_owned()));
                    }
                    *state = PlaybackState {
                        failure: None,
                        track_id: Some(track.track.id.clone()),
                        duration_ms: track.track.duration_ms,
                        order: state.order,
                    };
                };
                self.spawn_completion_monitor(generation);
                Ok(())
            } => result,
        }
    }

    fn spawn_completion_monitor(self: &Arc<Self>, generation: u64) {
        let weak = Arc::downgrade(self);
        let mut changes = self.generation.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = changes.wait_for(|current| *current != generation) => return,
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                }
                let Some(music) = weak.upgrade() else {
                    return;
                };
                let result = match music.audio.snapshot() {
                    Ok(audio) if !audio.ended => continue,
                    Ok(_) => music.advance(generation).await,
                    Err(error) => Err(error),
                };
                if let Err(error) = result {
                    tracing::warn!(%error, "could not advance the QQ Music queue");
                    let mut state = music.playback.write().await;
                    if *music.generation.borrow() == generation {
                        state.failure =
                            Some(format!("Could not play the next QQ Music track: {error}"));
                    }
                }
                return;
            }
        });
    }

    async fn advance(self: &Arc<Self>, generation: u64) -> Result<()> {
        let state = self.playback.read().await;
        let current = self
            .audio
            .snapshot()?
            .track
            .map(|track| track.id)
            .or_else(|| state.track_id.clone());
        let order = state.order;
        drop(state);
        let tracks = self.library.read().await.tracks.clone();
        let next = match order {
            PlaybackOrder::Sequential => current
                .as_ref()
                .and_then(|current| tracks.iter().position(|track| &track.id == current))
                .and_then(|index| tracks.get(index + 1)),
            PlaybackOrder::RepeatOne => current
                .as_ref()
                .and_then(|current| tracks.iter().find(|track| &track.id == current)),
            PlaybackOrder::Shuffle => {
                let choices: Vec<&Track> = tracks
                    .iter()
                    .filter(|track| Some(track.id.as_str()) != current.as_deref())
                    .collect();
                (!choices.is_empty())
                    .then(|| choices[rand::rng().random_range(0..choices.len())])
                    .or_else(|| {
                        tracks
                            .iter()
                            .find(|track| Some(track.id.as_str()) == current.as_deref())
                    })
            }
        };
        if let Some(next) = next {
            self.load_track(&next.id, generation).await
        } else {
            Ok(())
        }
    }

    async fn play_url(&self, track: &QqTrack, session: &QqSession) -> Result<String> {
        let auth = &session.auth;
        let guid = uuid::Uuid::new_v4().simple().to_string();
        let qualities = [
            ("AI00", ".flac"),
            ("F000", ".flac"),
            ("M800", ".mp3"),
            ("M500", ".mp3"),
            ("C400", ".m4a"),
        ];
        let filenames: Vec<String> = qualities
            .iter()
            .map(|(prefix, extension)| format!("{prefix}{}{extension}", track.media_mid))
            .collect();
        let repeated = vec![track.track.id.clone(); filenames.len()];
        let response = self
            .http
            .post(API)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::COOKIE, &session.cookie)
            .json(&serde_json::json!({
                "comm": {
                    "uin": &auth.uin,
                    "format": "json",
                    "ct": 24,
                    "cv": 4747474,
                    "platform": "yqq.json",
                    "g_tk": auth.gtk,
                    "g_tk_new_20200303": auth.gtk,
                    "inCharset": "utf-8",
                    "outCharset": "utf-8",
                    "notice": 0,
                    "needNewCode": 1,
                    "authst": &auth.authst,
                    "tmeLoginType": auth.login_type
                },
                "req_0": {
                    "module": "music.vkey.GetVkey",
                    "method": "UrlGetVkey",
                    "param": {
                        "guid": guid,
                        "songmid": repeated,
                        "songtype": vec![0; filenames.len()],
                        "uin": &auth.uin,
                        "ctx": 0,
                        "filename": &filenames
                    }
                }
            }))
            .send()
            .await?;
        let data: VkeyData = QqResponse::read(response, "resolve QQ Music playback URL").await?;
        let path = filenames
            .iter()
            .find_map(|filename| {
                data.urls.iter().find(|info| {
                    info.filename == *filename && (!info.purl.is_empty() || !info.wifi_url.is_empty())
                })
            })
            .map(|info| {
                if info.purl.is_empty() {
                    info.wifi_url.clone()
                } else {
                    info.purl.clone()
                }
            })
            .ok_or_else(|| Error::Playback("QQ Music did not return a playable URL. Reconnect the account or check its playback rights.".to_owned()))?;
        if let Ok(url) = reqwest::Url::parse(&path) {
            return trusted_media_url(url).map(|url| url.into());
        }
        let base = data
            .sip
            .into_iter()
            .filter_map(|value| reqwest::Url::parse(&value).ok())
            .find_map(|url| trusted_media_url(url).ok())
            .unwrap_or_else(|| {
                reqwest::Url::parse("https://ws.stream.qqmusic.qq.com/")
                    .expect("the static QQ Music media origin is valid")
            });
        let url = base
            .join(&path)
            .map_err(|_| Error::InvalidData("QQ Music returned an invalid media URL".to_owned()))?;
        trusted_media_url(url).map(|url| url.into())
    }
}

fn trusted_media_url(url: reqwest::Url) -> Result<reqwest::Url> {
    let trusted_host = url
        .host_str()
        .is_some_and(|host| host == "qqmusic.qq.com" || host.ends_with(".qqmusic.qq.com"));
    if url.scheme() != "https" || !trusted_host {
        return Err(Error::InvalidData(
            "QQ Music returned an untrusted media URL".to_owned(),
        ));
    }
    Ok(url)
}

async fn read_bytes(
    response: reqwest::Response,
    operation: &'static str,
    limit: usize,
) -> Result<Vec<u8>> {
    let response = check(response, operation)?;
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(Error::InvalidData(format!(
            "{operation} exceeds the size limit"
        )));
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(Error::InvalidData(format!(
                "{operation} exceeds the size limit"
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
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

fn parse_cookie(cookie: &str) -> HashMap<String, String> {
    cookie
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .filter(|(key, _)| !key.trim().is_empty())
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

fn render_cookie(fields: HashMap<String, String>) -> String {
    let mut fields: Vec<_> = fields.into_iter().collect();
    fields.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    fields
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn renewal_due(cookie: &str) -> bool {
    let fields = parse_cookie(cookie);
    let has_refresh = fields
        .get("psrf_qqrefresh_token")
        .or_else(|| fields.get("wxrefresh_token"))
        .is_some_and(|value| !value.is_empty());
    let Some(created) = fields
        .get("psrf_musickey_createtime")
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    has_refresh && now.saturating_sub(created) >= RENEW_AFTER.as_secs()
}

#[derive(serde::Serialize)]
struct QqAuth {
    uin: String,
    format: &'static str,
    ct: u8,
    cv: u8,
    authst: String,
    #[serde(rename = "tmeLoginType")]
    login_type: u8,
    #[serde(skip)]
    gtk: u32,
}

#[derive(Default, serde::Deserialize)]
struct RenewData {
    #[serde(default)]
    musickey: String,
    #[serde(default)]
    openid: String,
    #[serde(default)]
    unionid: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    expired_at: u64,
    #[serde(default)]
    musicid: u64,
    #[serde(default)]
    str_musicid: String,
    #[serde(default)]
    refresh_key: String,
    #[serde(default, rename = "musickeyCreateTime")]
    musickey_create_time: u64,
    #[serde(default, rename = "encryptUin")]
    encrypt_uin: String,
    #[serde(default, rename = "loginType")]
    login_type: u8,
}

impl RenewData {
    fn apply(self, fields: &mut HashMap<String, String>, login_type: u8) {
        fields.insert("qm_keyst".to_owned(), self.musickey.clone());
        fields.insert("qqmusic_key".to_owned(), self.musickey);
        fields.insert(
            "tmeLoginType".to_owned(),
            if self.login_type == 0 {
                login_type
            } else {
                self.login_type
            }
            .to_string(),
        );
        let music_id = if self.str_musicid.is_empty() {
            self.musicid.to_string()
        } else {
            self.str_musicid.trim_start_matches('o').to_owned()
        };
        if music_id != "0" {
            fields.insert("uin".to_owned(), music_id.clone());
            fields.insert("qm_str_musicid".to_owned(), music_id.clone());
            if login_type == 1 {
                fields.insert("wxuin".to_owned(), music_id);
            }
        }
        if !self.openid.is_empty() {
            let field = if login_type == 1 {
                "wxopenid"
            } else {
                "psrf_qqopenid"
            };
            fields.insert(field.to_owned(), self.openid);
        }
        if !self.refresh_token.is_empty() {
            let field = if login_type == 1 {
                "wxrefresh_token"
            } else {
                "psrf_qqrefresh_token"
            };
            fields.insert(field.to_owned(), self.refresh_token);
        }
        if !self.access_token.is_empty() {
            fields.insert("psrf_qqaccess_token".to_owned(), self.access_token);
        }
        if !self.unionid.is_empty() {
            fields.insert("psrf_qqunionid".to_owned(), self.unionid);
        }
        if !self.refresh_key.is_empty() {
            fields.insert("qm_refresh_key".to_owned(), self.refresh_key);
        }
        if !self.encrypt_uin.is_empty() {
            fields.insert("euin".to_owned(), self.encrypt_uin);
        }
        if self.expired_at > 0 {
            fields.insert(
                "psrf_access_token_expiresAt".to_owned(),
                self.expired_at.to_string(),
            );
        }
        if self.musickey_create_time > 0 {
            fields.insert(
                "psrf_musickey_createtime".to_owned(),
                self.musickey_create_time.to_string(),
            );
        }
    }
}

#[derive(serde::Deserialize)]
struct FeedData {
    #[serde(default, rename = "v_shelf")]
    shelves: Vec<FeedShelf>,
}

#[derive(serde::Deserialize)]
struct FeedShelf {
    #[serde(default, rename = "v_niche")]
    niches: Vec<FeedNiche>,
}

#[derive(serde::Deserialize)]
struct FeedNiche {
    #[serde(default, rename = "v_card")]
    cards: Vec<FeedCard>,
}

#[derive(serde::Deserialize)]
struct FeedCard {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
}

#[derive(serde::Deserialize)]
struct DailyData {
    #[serde(default, rename = "songlist")]
    songs: Vec<TrackWire>,
}

#[derive(Default, serde::Deserialize)]
struct TrackWire {
    #[serde(default)]
    mid: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "singer", alias = "singers")]
    singers: Vec<ArtistWire>,
    #[serde(default)]
    album: AlbumWire,
    #[serde(default)]
    interval: u64,
    #[serde(default)]
    file: FileWire,
}

#[derive(serde::Deserialize)]
struct ArtistWire {
    #[serde(default)]
    name: String,
}

#[derive(Default, serde::Deserialize)]
struct AlbumWire {
    #[serde(default)]
    mid: String,
    #[serde(default)]
    name: String,
}

#[derive(Default, serde::Deserialize)]
struct FileWire {
    #[serde(default)]
    media_mid: String,
}

#[derive(serde::Deserialize)]
struct VkeyData {
    #[serde(default)]
    sip: Vec<String>,
    #[serde(default, rename = "midurlinfo")]
    urls: Vec<VkeyInfo>,
}

#[derive(serde::Deserialize)]
struct QqResponse {
    #[serde(default)]
    code: i64,
    #[serde(alias = "feed", alias = "daily", alias = "lyric", alias = "req_0")]
    request: Option<QqBlock>,
}

#[derive(serde::Deserialize)]
struct QqBlock {
    #[serde(default)]
    code: i64,
    data: Option<serde_json::Value>,
}

impl QqResponse {
    async fn read<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
        operation: &'static str,
    ) -> Result<T> {
        let response = check(response, operation)?;
        let bytes = response.bytes().await?;
        let response: Self = serde_json::from_slice(&bytes).map_err(|error| {
            // Serde's display message can contain private response values.
            let reason = match error.classify() {
                serde_json::error::Category::Data => {
                    "JSON fields do not match the expected response"
                }
                serde_json::error::Category::Syntax => "response is not valid JSON",
                serde_json::error::Category::Eof => "response is empty or incomplete JSON",
                serde_json::error::Category::Io => "response could not be read",
            };
            Error::InvalidData(format!(
                "{operation}: {reason} (line {}, column {})",
                error.line(),
                error.column()
            ))
        })?;
        if response.code != 0 {
            return Err(Error::InvalidData(format!(
                "{operation}: QQ Music returned code {}",
                response.code
            )));
        }
        let missing = if response.request.is_none() {
            "response block is missing"
        } else {
            "response data is missing"
        };
        let data = match response.request {
            Some(block) => {
                if block.code != 0 {
                    return Err(Error::InvalidData(format!(
                        "{operation}: QQ Music returned code {}",
                        block.code
                    )));
                }
                block.data
            }
            None => None,
        };
        // Optional payloads (lyrics) retain the protocol's successful absence semantics.
        let reason = if data.is_none() {
            missing
        } else {
            "data fields do not match the expected response"
        };
        serde_json::from_value(data.unwrap_or(serde_json::Value::Null))
            .map_err(|_| Error::InvalidData(format!("{operation}: {reason}")))
    }
}

#[derive(serde::Deserialize)]
struct VkeyInfo {
    #[serde(default)]
    filename: String,
    #[serde(default)]
    purl: String,
    #[serde(default, rename = "wifiurl")]
    wifi_url: String,
}

#[derive(serde::Deserialize)]
struct LyricData {
    lyric: Option<String>,
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn media_download_enforces_declared_and_streamed_size_limits() {
        for declared in [false, true] {
            let mut response = http::Response::builder().status(200);
            if declared {
                response = response.header("content-length", "5");
            }
            let body = if declared {
                reqwest::Body::from("12345")
            } else {
                reqwest::Body::wrap_stream(futures_util::stream::iter([
                    Ok::<_, std::io::Error>("123"),
                    Ok("45"),
                ]))
            };
            let response = reqwest::Response::from(response.body(body).unwrap());
            assert_eq!(response.content_length().is_some(), declared);
            assert!(
                matches!(super::read_bytes(response, "audio", 4).await, Err(crate::Error::InvalidData(message)) if message.contains("size limit"))
            );
        }
        let response =
            reqwest::Response::from(http::Response::builder().status(200).body("1234").unwrap());
        assert_eq!(
            super::read_bytes(response, "audio", 4).await.unwrap(),
            b"1234"
        );
        let response = reqwest::Response::from(
            http::Response::builder()
                .status(403)
                .body("secret response")
                .unwrap(),
        );
        let error = super::read_bytes(response, "audio", 4).await.unwrap_err();
        assert!(matches!(error, crate::Error::Status { .. }));
        assert!(!error.to_string().contains("secret response"));
    }
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        DailyData, FeedData, QqResponse, RenewData, VkeyData, parse_cookie, renewal_due,
        trusted_media_url,
    };

    #[tokio::test]
    async fn dropping_runtime_releases_monitor() {
        let music = std::sync::Arc::new(
            super::QqMusic::new(vesper_credentials::QqMusicCredentials {
                cookie: String::new(),
            })
            .unwrap(),
        );
        let weak = std::sync::Arc::downgrade(&music);
        music.spawn_completion_monitor(0);
        drop(music);
        tokio::task::yield_now().await;
        assert!(weak.upgrade().is_none());
    }

    #[tokio::test]
    async fn newer_action_cancels_a_waiting_load() {
        let music = std::sync::Arc::new(
            super::QqMusic::new(vesper_credentials::QqMusicCredentials {
                cookie: String::new(),
            })
            .unwrap(),
        );
        let operation = music.playback_operation.lock().await;
        let load = music.load_track("old", 0);
        tokio::pin!(load);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut load)
                .await
                .is_err()
        );
        music.pause().await.unwrap();
        let error = tokio::time::timeout(std::time::Duration::from_millis(100), load)
            .await
            .unwrap()
            .unwrap_err();
        assert!(error.to_string().contains("cancelled"));
        drop(operation);
        assert!(
            music
                .play("latest")
                .await
                .unwrap_err()
                .to_string()
                .contains("unknown QQ Music track id")
        );
        music.shutdown().await;
        assert!(
            music
                .play("latest")
                .await
                .unwrap_err()
                .to_string()
                .contains("closed")
        );
    }

    #[tokio::test]
    async fn resume_reloads_ended_audio() {
        let music = std::sync::Arc::new(
            super::QqMusic::new(vesper_credentials::QqMusicCredentials {
                cookie: String::new(),
            })
            .unwrap(),
        );
        music.playback.write().await.track_id = Some("current".to_owned());
        // No live sink remains: resume must enter the load path, not return a silent success.
        assert!(
            music
                .resume()
                .await
                .unwrap_err()
                .to_string()
                .contains("unknown QQ Music track id")
        );
    }

    #[tokio::test]
    async fn automatic_failure_is_visible() {
        let music = super::QqMusic::new(vesper_credentials::QqMusicCredentials {
            cookie: String::new(),
        })
        .unwrap();
        music.playback.write().await.track_id = Some("current".to_owned());
        music.playback.write().await.order = super::PlaybackOrder::RepeatOne;
        music.library.write().await.tracks.push(super::Track {
            id: "current".to_owned(),
            name: "Current".to_owned(),
            artists: Vec::new(),
            album: String::new(),
            duration_ms: 1000,
            added_at: String::new(),
            cover_key: None,
        });
        // The missing track entry deterministically fails automatic loading before any network I/O.
        let music = std::sync::Arc::new(music);
        music.spawn_completion_monitor(0);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if let Err(error) = music.playback().await {
                    assert!(error.to_string().contains("unknown QQ Music track id"));
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("automatic failure must be visible through playback reads");
    }

    #[tokio::test]
    async fn rpc_success_aliases_and_optional_lyrics() {
        for key in ["request", "feed", "daily", "lyric", "req_0"] {
            let body = serde_json::json!({"code":0, key:{"code":0,"data":{"musickey":"key","musicid":123}}}).to_string();
            let response = http::Response::new(body);
            let data: RenewData = QqResponse::read(response.into(), "QQ test").await.unwrap();
            assert_eq!(data.musickey, "key");
        }
        for body in [
            r#"{"code":0}"#,
            r#"{"lyric":{"code":0}}"#,
            r#"{"lyric":{"data":{}}}"#,
        ] {
            let data: Option<super::LyricData> =
                QqResponse::read(http::Response::new(body).into(), "lyrics")
                    .await
                    .unwrap();
            assert!(data.and_then(|data| data.lyric).is_none());
        }
        let error = QqResponse::read::<Option<super::LyricData>>(
            http::Response::new(r#"{"lyric":{"code":2000}}"#).into(),
            "lyrics",
        )
        .await
        .err()
        .unwrap();
        assert!(error.to_string().contains("code 2000"));
    }

    #[tokio::test]
    async fn playback_rejections() {
        for body in [
            r#"{"code":2000}"#,
            r#"{"code":0,"req_0":{"code":2000}}"#,
            r#"{"code":0,"req_0":{"code":2000,"data":null}}"#,
            r#"{"code":0,"req_0":{"code":2000,"data":{"sip":"private-value"}}}"#,
        ] {
            let response = http::Response::new(body);
            let error =
                QqResponse::read::<VkeyData>(response.into(), "resolve QQ Music playback URL")
                    .await
                    .err()
                    .unwrap()
                    .to_string();
            assert!(error.contains("resolve QQ Music playback URL"));
            assert!(error.contains("code 2000"));
            assert!(!error.contains("private-value"));
        }
    }

    #[tokio::test]
    async fn playback_response_errors() {
        for (body, reason) in [
            ("<html>private-value</html>", "not valid JSON"),
            ("", "empty or incomplete JSON"),
            (r#"{"code":"private-value"}"#, "JSON fields do not match"),
        ] {
            let response = http::Response::new(body);
            let error =
                QqResponse::read::<VkeyData>(response.into(), "resolve QQ Music playback URL")
                    .await
                    .err()
                    .unwrap()
                    .to_string();
            assert!(error.contains(reason));
            assert!(error.contains("resolve QQ Music playback URL"));
            assert!(!error.contains("private-value"));
        }
        let response = http::Response::builder()
            .status(503)
            .body("private-value")
            .unwrap();
        assert!(matches!(
            QqResponse::read::<VkeyData>(response.into(), "resolve QQ Music playback URL").await,
            Err(crate::Error::Status {
                status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn playback_response_data() {
        let response = http::Response::new(
            r#"{"code":0,"req_0":{"code":0,"data":{"sip":["https://ws.stream.qqmusic.qq.com/"],"midurlinfo":[{"filename":"M500song.mp3","purl":"song.mp3"}]}}}"#,
        );
        let data = QqResponse::read::<VkeyData>(response.into(), "resolve QQ Music playback URL")
            .await
            .unwrap();
        assert_eq!(data.urls[0].purl, "song.mp3");
        for body in [
            r#"{"code":0}"#,
            r#"{"code":0,"req_0":{"code":0}}"#,
            r#"{"code":0,"req_0":{"code":0,"data":null}}"#,
            r#"{"code":0,"req_0":{"code":0,"data":{"sip":"private-value"}}}"#,
        ] {
            let response = http::Response::new(body);
            let error =
                QqResponse::read::<VkeyData>(response.into(), "resolve QQ Music playback URL")
                    .await
                    .err()
                    .unwrap()
                    .to_string();
            assert!(error.contains("resolve QQ Music playback URL"));
            assert!(!error.contains("private-value"));
        }
    }

    #[tokio::test]
    async fn parses_daily_card_and_playlist() {
        let response: FeedData = QqResponse::read(http::Response::new(
            r#"{"feed":{"code":0,"data":{"v_shelf":[{"v_niche":[{"v_card":[{"id":"123","title":"每日30首"}]}]}]}}}"#,
        ).into(), "QQ Music test").await.unwrap();
        assert_eq!(response.shelves[0].niches[0].cards[0].id, "123");

        let response: DailyData = QqResponse::read(http::Response::new(
            r#"{"daily":{"code":0,"data":{"songlist":[{"mid":"song-mid","name":"Song","singer":[{"name":"Artist"}],"album":{"mid":"album-mid","name":"Album"},"interval":180,"file":{"media_mid":"media-mid"}}]}}}"#,
        ).into(), "QQ Music test").await.unwrap();
        assert_eq!(response.songs[0].mid, "song-mid");
    }

    #[test]
    fn renews_a_refreshable_session_after_twenty_hours() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let fresh = format!(
            "uin=1; qm_keyst=key; psrf_qqrefresh_token=refresh; psrf_musickey_createtime={}",
            now - 19 * 60 * 60
        );
        let stale = format!(
            "uin=1; qm_keyst=key; psrf_qqrefresh_token=refresh; psrf_musickey_createtime={}",
            now - 20 * 60 * 60
        );
        assert!(!renewal_due(&fresh));
        assert!(renewal_due(&stale));
        assert!(!renewal_due(
            "uin=1; qm_keyst=key; psrf_musickey_createtime=1"
        ));
    }

    #[test]
    fn rotates_all_returned_session_fields() {
        let mut fields =
            parse_cookie("uin=1; qm_keyst=old; qqmusic_key=old; psrf_qqrefresh_token=old-refresh");
        RenewData {
            musickey: "new".to_owned(),
            refresh_token: "new-refresh".to_owned(),
            musickey_create_time: 123,
            ..RenewData::default()
        }
        .apply(&mut fields, 2);
        assert_eq!(fields.get("qm_keyst").map(String::as_str), Some("new"));
        assert_eq!(
            fields.get("psrf_qqrefresh_token").map(String::as_str),
            Some("new-refresh")
        );
        assert_eq!(
            fields.get("psrf_musickey_createtime").map(String::as_str),
            Some("123")
        );
    }

    #[test]
    fn rejects_untrusted_media_urls() {
        assert!(
            trusted_media_url(
                reqwest::Url::parse("https://ws.stream.qqmusic.qq.com/song.mp3").unwrap()
            )
            .is_ok()
        );
        assert!(
            trusted_media_url(reqwest::Url::parse("http://127.0.0.1/song.mp3").unwrap()).is_err()
        );
    }
}
