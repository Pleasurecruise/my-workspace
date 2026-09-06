use super::{Identity, Roles, TokenKind, passport, request, signature};
use crate::{Account, Game, transport};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use vesper_credentials::games::{RecordDevice, Session};

pub const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 13; Pixel 5 Build/TQ3A.230901.001; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/118.0.0.0 Mobile Safari/537.36 miHoYoBBS/2.90.1";

#[derive(Clone)]
pub struct RecordSession {
    pub(crate) session: Session,
    pub(super) cookies: BTreeMap<String, String>,
    pub(super) device: String,
    fingerprint: String,
    updated_at: i64,
    accounts: std::sync::Arc<tokio::sync::Mutex<BTreeMap<String, Account>>>,
    pub(super) challenges: std::sync::Arc<tokio::sync::Mutex<BTreeMap<String, String>>>,
    pub(super) verification_traces: std::sync::Arc<tokio::sync::Mutex<BTreeMap<String, String>>>,
}

#[derive(Deserialize)]
struct CookieToken {
    uid: String,
    cookie_token: String,
}

#[derive(Deserialize)]
struct Fingerprint {
    code: i64,
    device_fp: String,
}

impl RecordSession {
    pub(crate) fn authkey_request(&self, body: String) -> Result<reqwest::RequestBuilder, String> {
        let Session::Mihoyo {
            account_id,
            stoken,
            mid,
            ..
        } = &self.session
        else {
            return Err("miHoYo is not connected".into());
        };
        // Hutao BindingClient2: SToken only, Gen1/LK2 signing and the app Referer.
        Ok(transport::client()?
            .post("https://api-takumi.mihoyo.com/binding/api/genAuthKey")
            .header(
                "Cookie",
                format!("stuid={account_id}; stoken={stoken}; mid={mid}"),
            )
            .header(
                "DS",
                signature("sidQFEglajEz7FA0Aj7HQPV88zpf17SO", None, ""),
            )
            .header("x-rpc-app_version", "2.95.1")
            .header("x-rpc-client_type", "5")
            .header("x-rpc-device_id", &self.device)
            .header("x-rpc-device_fp", &self.fingerprint)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) miHoYoBBS/2.95.1",
            )
            .header("Referer", "https://app.mihoyo.com")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(body))
    }

    pub(crate) fn matches(&self, session: &Session) -> bool {
        let (
            Session::Mihoyo {
                stoken: previous,
                account_id: previous_id,
                ..
            },
            Session::Mihoyo {
                stoken, account_id, ..
            },
        ) = (&self.session, session)
        else {
            return false;
        };
        previous == stoken
            && previous_id == account_id
            && transport::now() - self.updated_at < 3 * 24 * 60 * 60
    }

    pub(crate) async fn create(session: &Session) -> Result<Self, String> {
        let Session::Mihoyo {
            account_id,
            mid,
            stoken,
            ltoken,
            record,
            ..
        } = session
        else {
            return Err("miHoYo is not connected".into());
        };
        let client = transport::client()?;
        let device = record
            .as_ref()
            .map(|record| record.id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let identity = Identity {
            aid: account_id.clone(),
            mid: mid.clone(),
        };
        let token: CookieToken = request(passport(
            &client,
            TokenKind::Cookie,
            &device,
            stoken,
            &identity,
        ))
        .await
        .map_err(|error| format!("Game-record Cookie exchange: {error}"))?;
        if token.uid != *account_id || token.cookie_token.is_empty() {
            return Err("Game-record Cookie exchange returned an invalid account".into());
        }
        let record = if let Some(record) = record
            .as_ref()
            .filter(|record| transport::now() - record.updated_at < 3 * 24 * 60 * 60)
        {
            record.clone()
        } else {
            let product: String = (0..6)
                .map(|_| {
                    char::from(
                        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"[rand::rng().random_range(0..36)],
                    )
                })
                .collect();
            let mut fields: BTreeMap<String, serde_json::Value> =
                serde_json::from_str(include_str!("device.json"))
                    .map_err(|_| "Invalid game-record device profile")?;
            fields.insert("productName".into(), serde_json::json!(product));
            fields.insert(
                "deviceInfo".into(),
                serde_json::json!(format!(
                    "google/{product}/redfin:13/TQ3A.230901.001/2311.40000.5.0:user/release-keys"
                )),
            );
            let fields = serde_json::to_string(&fields)
                .map_err(|_| "Could not encode the device profile")?;
            let timestamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
            let previous = record
                .as_ref()
                .map(|record| record.fingerprint.as_str())
                .unwrap_or("0000000000000");
            let fingerprint: Fingerprint = request(
                client
                    .post("https://public-data-api.mihoyo.com/device-fp/api/getFp")
                    .json(&serde_json::json!({
                        "device_id": hex::encode(rand::random::<[u8; 8]>()),
                        "seed_id": uuid::Uuid::new_v4().to_string(),
                        "seed_time": timestamp.to_string(),
                        "platform": "2",
                        "device_fp": previous,
                        "app_name": "bbs_cn",
                        "ext_fields": fields,
                        "bbs_device_id": device
                    })),
            )
            .await
            .map_err(|error| format!("Game-record device registration: {error}"))?;
            if fingerprint.code != 200 || fingerprint.device_fp.is_empty() {
                return Err("miHoYo rejected device registration. Game records are unavailable. Please try again manually later.".into());
            }
            RecordDevice {
                id: device.clone(),
                fingerprint: fingerprint.device_fp,
                updated_at: transport::now(),
            }
        };
        let cookies = BTreeMap::from([
            ("account_id".into(), account_id.clone()),
            ("cookie_token".into(), token.cookie_token),
            ("ltuid".into(), account_id.clone()),
            ("ltoken".into(), ltoken.clone()),
        ]);
        let mut session = session.clone();
        if let Session::Mihoyo { record: saved, .. } = &mut session {
            *saved = Some(record.clone());
        }
        vesper_credentials::games::accounts::update(|accounts| {
            let Some(Session::Mihoyo {
                stoken: current,
                record: saved,
                ..
            }) = accounts.sessions.get_mut(account_id)
            else {
                return Err(vesper_credentials::CredentialError::InvalidStore);
            };
            if current != stoken {
                return Err(vesper_credentials::CredentialError::InvalidValue(
                    "miHoYo session",
                    "login changed; retry the request",
                ));
            }
            *saved = Some(record.clone());
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        Ok(Self {
            session,
            cookies,
            device,
            fingerprint: record.fingerprint,
            updated_at: record.updated_at,
            accounts: Default::default(),
            challenges: Default::default(),
            verification_traces: Default::default(),
        })
    }

    pub(crate) fn request(
        &self,
        game: Game,
        url: reqwest::Url,
        body: Option<&str>,
    ) -> Result<reqwest::RequestBuilder, String> {
        let mut query: Vec<_> = url
            .query_pairs()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        query.sort();
        let client = transport::client()?;
        let request = match body {
            Some(body) => client
                .post(url)
                .header("Content-Type", "application/json")
                .body(body.to_owned()),
            None => client.get(url),
        };
        let (version, agent) = if matches!(game, Game::Genshin | Game::StarRail) {
            (
                "2.95.1",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) miHoYoBBS/2.95.1",
            )
        } else {
            ("2.90.1", USER_AGENT)
        };
        let request = if game == Game::StarRail {
            request
                .header("x-rpc-tool_verison", "v4.5.0")
                .header("x-rpc-page", "v4.5.0_#/rpg")
        } else {
            request
        };
        Ok(request
            .header("User-Agent", agent)
            .header("Accept", "application/json")
            .header(
                "Cookie",
                self.cookies
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
            .header(
                "DS",
                signature(
                    "xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs",
                    Some(body.unwrap_or("")),
                    &query.join("&"),
                ),
            )
            .header("Referer", "https://webstatic.mihoyo.com/")
            .header("x-rpc-app_version", version)
            .header("x-rpc-client_type", "5")
            .header("x-rpc-device_id", &self.device)
            .header("x-rpc-device_fp", &self.fingerprint))
    }

    pub(crate) async fn note_request(
        &self,
        game: Game,
        account: &Account,
    ) -> Result<reqwest::RequestBuilder, String> {
        let mut url = reqwest::Url::parse(super::notes_endpoint(game)?)
            .map_err(|_| "Invalid game endpoint")?;
        url.query_pairs_mut()
            .append_pair("role_id", &account.uid)
            .append_pair("server", &account.region);
        let mut request = self.request(game, url, None)?;
        if game == Game::Genshin {
            request = request.header("x-rpc-tool_verison", "v5.0.1-ys");
        }
        if let Some(challenge) = self.challenges.lock().await.remove(game.key()) {
            request = request.header("x-rpc-challenge", challenge);
        }
        Ok(request)
    }

    pub(crate) async fn remember_verification_trace(
        &self,
        game: Game,
        code: i64,
        headers: &reqwest::header::HeaderMap,
    ) {
        let mut traces = self.verification_traces.lock().await;
        traces.remove(game.key());
        if matches!(code, 1034 | 10035 | 5003 | 10041 | 10053)
            && let Some(trace) = headers
                .get("x-trace-id")
                .and_then(|value| value.to_str().ok())
            && !trace.is_empty()
            && trace.len() <= 256
            && trace.bytes().all(|byte| byte.is_ascii_graphic())
        {
            traces.insert(game.key().into(), trace.into());
        }
    }

    pub(crate) async fn account(&self, game: Game) -> Result<Account, String> {
        let (biz, region) = match game {
            Game::Genshin => ("hk4e_cn", "cn_gf01"),
            Game::StarRail => ("hkrpg_cn", "prod_gf_cn"),
            Game::Zzz => ("nap_cn", "prod_gf_cn"),
            _ => return Err("Unsupported miHoYo game".into()),
        };
        let mut accounts = self.accounts.lock().await;
        if let Some(account) = accounts.get(game.key()) {
            return Ok(account.clone());
        }
        let roles: Roles = request(
            transport::client()?
                .get("https://passport-api.mihoyo.com/binding/api/getUserGameRolesByCookieToken")
                .query(&[("game_biz", biz)])
                .header(
                    "Cookie",
                    self.cookies
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>()
                        .join("; "),
                )
                .header("Referer", "https://act.mihoyo.com/"),
        )
        .await
        .map_err(|error| format!("Game-record role lookup: {error}"))?;
        let account = roles.select(game, region)?;
        accounts.insert(game.key().into(), account.clone());
        Ok(account)
    }

    pub async fn page(&self, game: Game) -> Result<VerificationPage, String> {
        let account = self.account(game).await?;
        let url = match game {
            Game::Genshin => "https://webstatic.mihoyo.com/app/community-game-records/?game_id=2",
            Game::StarRail => "https://webstatic.mihoyo.com/app/community-game-records/?game_id=6",
            Game::Zzz => "https://act.mihoyo.com/app/mihoyo-zzz-game-record/m.html?game_id=8",
            _ => return Err("This game does not use Miyoushe verification".into()),
        };
        Ok(VerificationPage {
            record: self.clone(),
            account,
            url,
        })
    }
}

pub struct VerificationPage {
    record: RecordSession,
    account: Account,
    pub url: &'static str,
}

#[derive(Deserialize)]
pub struct BridgeMessage {
    pub method: String,
    pub callback: Option<String>,
    pub payload: Option<BridgePayload>,
}

#[derive(Deserialize)]
pub struct BridgePayload {
    pub page: Option<String>,
    query: Option<serde_json::Value>,
    body: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct BridgeResult {
    retcode: i64,
    message: &'static str,
    data: serde_json::Value,
}

impl VerificationPage {
    pub fn cookies(&self) -> &BTreeMap<String, String> {
        &self.record.cookies
    }

    pub fn respond(&self, message: &BridgeMessage) -> Result<Option<BridgeResult>, String> {
        let data = match message.method.as_str() {
            "getCookieInfo" => serde_json::to_value(&self.record.cookies)
                .map_err(|_| "Could not prepare verification cookies")?,
            "getCookieToken" => {
                serde_json::json!({"cookie_token":self.record.cookies.get("cookie_token")})
            }
            "getHTTPRequestHeaders" => {
                serde_json::json!({"x-rpc-client_type":"5","x-rpc-app_version":"2.90.1","x-rpc-device_fp":self.record.fingerprint,"x-rpc-device_id":self.record.device})
            }
            "getUserInfo" => {
                serde_json::json!({"id":self.account.uid,"gender":"","nickname":self.account.name,"introduce":"","avatar_url":""})
            }
            "getStatusBarHeight" => serde_json::json!({"statusBarHeight":0}),
            "getCurrentLocale" => serde_json::json!({"language":"zh-cn","timeZone":"GMT+8"}),
            "getDS" => {
                serde_json::json!({"DS":signature("t0qEgfub6cvueAPgR5m9aQWWVciEer7v", None, "")})
            }
            "getDS2" => {
                let payload = message
                    .payload
                    .as_ref()
                    .ok_or("Missing verification signing payload")?;
                let query: BTreeMap<String, serde_json::Value> = match &payload.query {
                    None | Some(serde_json::Value::Null) => BTreeMap::new(),
                    Some(serde_json::Value::String(query)) => {
                        serde_json::from_str::<Option<BTreeMap<String, serde_json::Value>>>(query)
                            .map_err(|_| "Invalid verification signing query")?
                            .unwrap_or_default()
                    }
                    Some(serde_json::Value::Object(query)) => query.clone().into_iter().collect(),
                    _ => return Err("Invalid verification signing query".into()),
                };
                let query = query
                    .iter()
                    .map(|(key, value)| match value {
                        serde_json::Value::String(value) => format!("{key}={value}"),
                        _ => format!("{key}={value}"),
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                let body = match &payload.body {
                    None | Some(serde_json::Value::Null) => String::new(),
                    Some(serde_json::Value::String(body)) => body.clone(),
                    Some(body) => body.to_string(),
                };
                serde_json::json!({"DS":signature("xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs", Some(&body), &query)})
            }
            _ => return Ok(None),
        };
        Ok(Some(BridgeResult {
            retcode: 0,
            message: "",
            data,
        }))
    }
}

#[cfg(test)]
#[path = "../../tests/unit/mihoyo/record.rs"]
pub(super) mod tests;
