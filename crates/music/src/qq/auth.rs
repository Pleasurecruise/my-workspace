use super::{API, QqMusic, QqResponse, REFERER};
use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RENEW_AFTER: Duration = Duration::from_secs(20 * 60 * 60);
const RENEW_RETRY: Duration = Duration::from_secs(60 * 60);

pub(super) struct QqSession {
    pub(super) cookie: String,
    pub(super) auth: QqAuth,
}

impl QqMusic {
    pub(super) async fn session(&self) -> Result<QqSession> {
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
        let login_type = parse_login_type(&fields, &key);
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
        Ok(render_cookie(&fields))
    }

    fn auth(cookie: &str) -> QqAuth {
        let cookies = parse_cookie(cookie);
        let declared_login_type = cookies
            .get("tmeLoginType")
            .or_else(|| cookies.get("login_type"))
            .map(String::as_str);
        // WeChat logins prefer their own uin field over the QQ identities.
        let identity_fields = if declared_login_type == Some("1") {
            ["wxuin", "uin", "qqmusic_uin", "p_uin"]
        } else {
            ["uin", "qqmusic_uin", "wxuin", "p_uin"]
        };
        let uin = identity_fields
            .iter()
            .find_map(|key| cookies.get(*key).map(String::as_str))
            .unwrap_or_default()
            .trim_start_matches('o')
            .trim_start_matches('0');
        let authst = ["qm_keyst", "qqmusic_key", "music_key", "wxskey"]
            .iter()
            .find_map(|key| cookies.get(*key).map(String::as_str))
            .unwrap_or_default();
        QqAuth {
            uin: if uin.is_empty() { "0" } else { uin }.to_owned(),
            format: "json",
            ct: 19,
            cv: 0,
            authst: authst.to_owned(),
            login_type: parse_login_type(&cookies, authst),
            gtk: hash33(authst, 5_381),
        }
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

pub(super) fn render_cookie(fields: &HashMap<String, String>) -> String {
    let mut fields: Vec<_> = fields.iter().collect();
    fields.sort_unstable_by(|left, right| left.0.cmp(right.0));
    fields
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

// QQ's 33-biased rolling hash; seeds vary per token (5381 for g_tk, 0 for QR polling).
pub(super) fn hash33(value: &str, seed: u32) -> u32 {
    value.bytes().fold(seed, |hash, byte| {
        hash.wrapping_add(hash.wrapping_shl(5))
            .wrapping_add(u32::from(byte))
    }) & 0x7fff_ffff
}

// The declared tmeLoginType wins; a W_X-prefixed session key marks a WeChat login.
fn parse_login_type(fields: &HashMap<String, String>, session_key: &str) -> u8 {
    fields
        .get("tmeLoginType")
        .or_else(|| fields.get("login_type"))
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(if session_key.starts_with("W_X") { 1 } else { 2 })
}

pub(super) fn read_time() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
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
    let now = read_time().as_secs();
    has_refresh && now.saturating_sub(created) >= RENEW_AFTER.as_secs()
}

#[derive(serde::Serialize)]
pub(super) struct QqAuth {
    pub(super) uin: String,
    format: &'static str,
    ct: u8,
    cv: u8,
    pub(super) authst: String,
    #[serde(rename = "tmeLoginType")]
    pub(super) login_type: u8,
    #[serde(skip)]
    pub(super) gtk: u32,
}

#[derive(Default, serde::Deserialize)]
pub(super) struct RenewData {
    #[serde(default)]
    pub(super) musickey: String,
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
    pub(super) fn apply(self, fields: &mut HashMap<String, String>, login_type: u8) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_renewal_age() {
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
    fn rotates_session_fields() {
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
}
