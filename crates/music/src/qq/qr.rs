use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::header::{COOKIE, LOCATION, REFERER as REFERER_HEADER, SET_COOKIE};

use super::{API, REFERER, RenewResponse, check, render_cookie};
use crate::{Error, Result};

const QR_API: &str = "https://ssl.ptlogin2.qq.com/ptqrshow";
const POLL_API: &str = "https://ssl.ptlogin2.qq.com/ptqrlogin";
const AUTHORIZE_API: &str = "https://graph.qq.com/oauth2.0/authorize";
const WEB_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/124.0.0.0 Safari/537.36";
const QR_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_QR_BYTES: usize = 1024 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QqQr {
    image: String,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum QqLoginStatus {
    Waiting,
    Scanned,
    Complete,
    Expired,
}

pub struct QqLogin {
    http: reqwest::Client,
    qrsig: String,
    expires_at: Instant,
}

impl QqLogin {
    pub async fn start() -> Result<(Self, QqQr)> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(35))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(WEB_AGENT)
            .build()?;
        let response = http
            .get(QR_API)
            .header(REFERER_HEADER, "https://xui.ptlogin2.qq.com/")
            .query(&[
                ("appid", "716027609"),
                ("e", "2"),
                ("l", "M"),
                ("s", "3"),
                ("d", "72"),
                ("v", "4"),
                ("daid", "383"),
                ("pt_3rd_aid", "100497308"),
            ])
            .send()
            .await?;
        let response = check(response, "create QQ Music login QR code")?;
        let cookies = response_cookies(response.headers(), HashMap::new());
        let qrsig = cookies
            .get("qrsig")
            .filter(|value| !value.is_empty())
            .cloned()
            .ok_or_else(|| {
                Error::Authentication("QQ login did not return a QR session".to_owned())
            })?;
        let bytes =
            super::read_bytes(response, "read QQ Music login QR code", MAX_QR_BYTES).await?;
        let image = format!(
            "data:image/png;base64,{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
        );
        Ok((
            Self {
                http,
                qrsig,
                expires_at: Instant::now() + QR_TTL,
            },
            QqQr { image },
        ))
    }

    pub async fn poll(
        &mut self,
    ) -> Result<(
        QqLoginStatus,
        Option<vesper_credentials::QqMusicCredentials>,
    )> {
        if Instant::now() >= self.expires_at {
            return Ok((QqLoginStatus::Expired, None));
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let action = format!("0-0-{now}");
        let response = self
            .http
            .get(POLL_API)
            .header(REFERER_HEADER, "https://xui.ptlogin2.qq.com/")
            .header(COOKIE, format!("qrsig={};", self.qrsig))
            .query(&[
                ("u1", "https://graph.qq.com/oauth2.0/login_jump"),
                ("ptqrtoken", &hash33(&self.qrsig, 0).to_string()),
                ("ptredirect", "0"),
                ("h", "1"),
                ("t", "1"),
                ("g", "1"),
                ("from_ui", "1"),
                ("ptlang", "2052"),
                ("action", &action),
                ("js_ver", "20102616"),
                ("js_type", "1"),
                ("pt_uistyle", "40"),
                ("aid", "716027609"),
                ("daid", "383"),
                ("pt_3rd_aid", "100497308"),
                ("has_onekey", "1"),
            ])
            .send()
            .await?;
        let response = check(response, "check QQ Music login QR code")?;
        let cookies = response_cookies(
            response.headers(),
            HashMap::from([("qrsig".to_owned(), self.qrsig.clone())]),
        );
        let text = response.text().await?;
        let Some(result) = parse_ptui(&text) else {
            return Ok((QqLoginStatus::Waiting, None));
        };
        tracing::debug!(code = %result.code, "QQ Music login QR status");
        match result.code.as_str() {
            "65" | "68" => Ok((QqLoginStatus::Expired, None)),
            "67" => Ok((QqLoginStatus::Scanned, None)),
            "0" => self
                .finish(&result.jump_url, cookies)
                .await
                .map(|credentials| (QqLoginStatus::Complete, Some(credentials))),
            _ => Ok((QqLoginStatus::Waiting, None)),
        }
    }

    async fn finish(
        &self,
        jump_url: &str,
        cookies: HashMap<String, String>,
    ) -> Result<vesper_credentials::QqMusicCredentials> {
        let jump = reqwest::Url::parse(jump_url).map_err(|_| {
            Error::Authentication("QQ login returned an invalid redirect".to_owned())
        })?;
        let trusted_host = jump
            .host_str()
            .is_some_and(|host| host == "qq.com" || host.ends_with(".qq.com"));
        if jump.scheme() != "https" || !trusted_host {
            return Err(Error::Authentication(
                "QQ login returned an untrusted redirect".to_owned(),
            ));
        }
        let fallback_uin = jump
            .query_pairs()
            .find_map(|(key, value)| (key == "uin").then(|| value.into_owned()))
            .unwrap_or_default();
        let response = self
            .http
            .get(jump)
            .header(REFERER_HEADER, "https://xui.ptlogin2.qq.com/")
            .header(COOKIE, cookie_header(&cookies))
            .send()
            .await?;
        let cookies = response_cookies(response.headers(), cookies);
        let skey = ["p_skey", "p_sKey", "skey", "pskey"]
            .iter()
            .find_map(|field| cookies.get(*field))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Error::Authentication("QQ login did not return p_skey".to_owned()))?;
        let response = self
            .http
            .post(AUTHORIZE_API)
            .header(REFERER_HEADER, "https://xui.ptlogin2.qq.com/")
            .header(COOKIE, cookie_header(&cookies))
            .form(&[
                ("response_type", "code".to_owned()),
                ("client_id", "100497308".to_owned()),
                (
                    "redirect_uri",
                    "https://y.qq.com/portal/wx_redirect.html?login_type=1&surl=https://y.qq.com/"
                        .to_owned(),
                ),
                ("scope", "get_user_info,get_app_friends".to_owned()),
                ("state", "state".to_owned()),
                ("switch", String::new()),
                ("from_ptlogin", "1".to_owned()),
                ("src", "1".to_owned()),
                ("update_auth", "1".to_owned()),
                ("openapi", "1010_1030".to_owned()),
                ("g_tk", hash33(skey, 5381).to_string()),
                (
                    "auth_time",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                        .to_string(),
                ),
                ("ui", uuid::Uuid::new_v4().to_string()),
            ])
            .send()
            .await?;
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                Error::Authentication("QQ authorization did not return a code".to_owned())
            })?;
        let code = reqwest::Url::parse(location)
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find_map(|(key, value)| (key == "code").then(|| value.into_owned()))
            })
            .ok_or_else(|| {
                Error::Authentication("QQ authorization did not return a code".to_owned())
            })?;
        let response = self
            .http
            .post(API)
            .header(REFERER_HEADER, REFERER)
            .json(&serde_json::json!({
                "comm": {
                    "ct": 11, "cv": 14090008, "v": 14090008, "chid": "10003505",
                    "os_ver": "15", "phonetype": "24122RKC7C", "tmeAppID": "qqmusic",
                    "nettype": "NETWORK_WIFI", "udid": "0", "OpenUDID": "0", "QIMEI36": "0",
                    "uin": "0", "tmeLoginType": 2
                },
                "request": {
                    "module": "QQConnectLogin.LoginServer",
                    "method": "QQLogin",
                    "param": { "code": code }
                }
            }))
            .send()
            .await?;
        let response = check(response, "complete QQ Music login")?;
        let response: RenewResponse = response.json().await?;
        let block = response.request.ok_or_else(|| {
            Error::Authentication("QQ Music login response is missing".to_owned())
        })?;
        if response.code != 0 || block.code != 0 || block.data.musickey.is_empty() {
            return Err(Error::Authentication("QQ Music rejected login".to_owned()));
        }
        let mut fields = HashMap::new();
        if !fallback_uin.is_empty() {
            fields.insert("uin".to_owned(), fallback_uin);
        }
        block.data.apply(&mut fields, 2);
        fields
            .entry("psrf_musickey_createtime".to_owned())
            .or_insert_with(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_string()
            });
        Ok(vesper_credentials::QqMusicCredentials {
            cookie: render_cookie(fields),
        })
    }
}

struct PtuiResult {
    code: String,
    jump_url: String,
}

fn parse_ptui(text: &str) -> Option<PtuiResult> {
    let start = text.find("ptuiCB(")? + "ptuiCB(".len();
    let mut fields = Vec::new();
    let mut field: Option<String> = None;
    let mut escaped = false;
    for character in text[start..].chars() {
        if let Some(value) = field.as_mut() {
            if escaped {
                value.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                fields.push(std::mem::take(value));
                field = None;
            } else {
                value.push(character);
            }
        } else if character == '\'' {
            field = Some(String::new());
        }
    }
    Some(PtuiResult {
        code: fields.first()?.to_owned(),
        jump_url: fields.get(2).cloned().unwrap_or_default(),
    })
}

fn response_cookies(
    headers: &reqwest::header::HeaderMap,
    mut cookies: HashMap<String, String>,
) -> HashMap<String, String> {
    for value in headers.get_all(SET_COOKIE) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        let Some((field, value)) = value
            .split(';')
            .next()
            .and_then(|part| part.split_once('='))
        else {
            continue;
        };
        if !field.trim().is_empty() && !value.trim().is_empty() {
            cookies.insert(field.trim().to_owned(), value.trim().to_owned());
        }
    }
    cookies
}

fn cookie_header(cookies: &HashMap<String, String>) -> String {
    let mut fields: Vec<_> = cookies.iter().collect();
    fields.sort_unstable_by(|left, right| left.0.cmp(right.0));
    fields
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn hash33(value: &str, seed: u32) -> u32 {
    value.bytes().fold(seed, |hash, byte| {
        hash.wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(u32::from(byte))
    }) & 0x7fff_ffff
}

#[cfg(test)]
mod tests {
    use super::{hash33, parse_ptui};

    #[test]
    fn parses_waiting_and_complete_callbacks() {
        let waiting = parse_ptui("\n ptuiCB( '66', '0', '', '0', 'waiting', '' );\n").unwrap();
        assert_eq!(waiting.code, "66");
        let complete = parse_ptui(
            "ptuiCB('0','0','https://ssl.ptlogin2.qq.com/check_sig?uin=123&pttype=1','0','ok','nick');",
        )
        .unwrap();
        assert_eq!(complete.code, "0");
        assert!(complete.jump_url.contains("uin=123"));
    }

    #[test]
    fn computes_qq_hash() {
        assert_eq!(hash33("abc", 0), 108_966);
    }
}
