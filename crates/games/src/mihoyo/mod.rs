use crate::{
    Account, Game, Meter, Notes, Pull, Task,
    login::{Pending, Poll},
    transport,
};
use md5::{Digest, Md5};
use serde::{Deserialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashSet};
use tracing::Instrument;
use vesper_credentials::games::Session;
pub(crate) mod record;
pub(crate) mod verification;

pub(crate) const VERIFICATION_MESSAGE: &str = "miHoYo requires security verification. Complete verification, close the window, then manually refresh this card.";

#[derive(Deserialize)]
struct Envelope {
    retcode: i64,
    data: Option<serde_json::Value>,
}

async fn request<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, String> {
    let response: Envelope = transport::json(request).await?;
    response.decode()
}

impl Envelope {
    fn note<T: DeserializeOwned>(self) -> Result<T, crate::NotesError> {
        if matches!(self.retcode, 1034 | 10035 | 5003 | 10041 | 10053) {
            return Err(crate::NotesError::VerificationRequired(self.retcode));
        }
        self.decode().map_err(crate::NotesError::Failed)
    }

    fn decode<T: DeserializeOwned>(self) -> Result<T, String> {
        if self.retcode != 0 {
            tracing::warn!(retcode = self.retcode, "miHoYo rejected a request");
        }
        match self.retcode {
            0 => {
                serde_json::from_value(self.data.ok_or("miHoYo returned no data")?).map_err(|_| {
                    "miHoYo returned an unsupported response. Please update Vesper.".to_owned()
                })
            }
            -100 | -262 | 10001 | -10001 => {
                Err("miHoYo login expired. Scan again in Settings.".to_owned())
            }
            10102 => Err("Enable real-time notes in Miyoushe for this game.".to_owned()),
            -3209 | -101 => Err("The verification challenge has expired. Close this window and verify again.".into()),
            -3202 => Err("miHoYo rejected the captcha result. Close this window and complete a new challenge.".into()),
            -201 => Err("miHoYo rejected the request parameters. The verification request is incompatible with the service.".into()),
            -205 => Err("miHoYo rejected the verification key. Close this window and request a new challenge.".into()),
            -110 => Err("Too many requests. Wait before trying again manually.".into()),
            -109 => Err("miHoYo rejected the application identifier. Please update Vesper.".into()),
            1034 | 10035 | 5003 | 10041 | 10053 => Err(VERIFICATION_MESSAGE.to_owned()),
            -3503 => Err(
                "miHoYo blocked this login because the device or network was flagged as risky. Verify that you can sign in to the Miyoushe mobile app, then create a new QR code."
                    .to_owned(),
            ),
            _ => Err("miHoYo rejected the request with an unrecognized error. Please try again manually later.".into()),
        }
    }
}

fn signature(salt: &str, body: Option<&str>, query: &str) -> String {
    let timestamp = transport::now();
    let random = uuid::Uuid::new_v4();
    let nonce = if salt == "xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs" {
        (100000 + random.as_u128() % 100000).to_string()
    } else {
        random.as_bytes()[..6]
            .iter()
            .map(|byte| char::from(b'a' + byte % 26))
            .collect()
    };
    let input = match body {
        Some(body) => format!("salt={salt}&t={timestamp}&r={nonce}&b={body}&q={query}"),
        None => format!("salt={salt}&t={timestamp}&r={nonce}"),
    };
    format!("{timestamp},{nonce},{:x}", Md5::digest(input.as_bytes()))
}

#[derive(Deserialize)]
struct Qr {
    url: String,
    ticket: String,
}
pub(crate) async fn begin() -> Result<(Pending, String), String> {
    let device = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )[..53]
        .to_owned();
    let qr: Qr = request(
        transport::client()?
            .post("https://passport-api.mihoyo.com/account/ma-cn-passport/app/createQRLogin")
            .header("x-rpc-app_id", "ddxf5dufpuyo")
            .header("x-rpc-client_type", "3")
            .header("User-Agent", "HYPContainer/1.1.4.133")
            .header("Accept", "application/json")
            .header("x-rpc-device_id", &device)
            .json(&serde_json::json!({})),
    )
    .await
    .map_err(|error| format!("QR creation: {error}"))?;
    let url = reqwest::Url::parse(&qr.url).map_err(|_| "Invalid miHoYo QR response")?;
    if url.scheme() != "https" || url.host_str() != Some("user.mihoyo.com") {
        return Err("Unexpected miHoYo QR origin".to_owned());
    }
    if qr.ticket.is_empty() {
        return Err("Missing miHoYo QR ticket".to_owned());
    }
    Ok((
        Pending {
            ticket: qr.ticket,
            device,
        },
        qr.url,
    ))
}

#[derive(Deserialize)]
#[serde(tag = "status")]
enum QrStatus {
    Created,
    Scanned,
    Confirmed {
        tokens: Vec<Token>,
        user_info: Identity,
        need_realperson: bool,
    },
}
#[derive(Deserialize)]
struct Token {
    token_type: u8,
    token: String,
}
#[derive(Deserialize)]
struct Identity {
    aid: String,
    mid: String,
}
#[derive(Deserialize)]
struct LToken {
    ltoken: String,
}

enum TokenKind {
    LToken,
    Cookie,
}

fn passport(
    client: &reqwest::Client,
    kind: TokenKind,
    device: &str,
    stoken: &str,
    identity: &Identity,
) -> reqwest::RequestBuilder {
    let url = match kind {
        TokenKind::LToken => "https://passport-api.mihoyo.com/account/auth/api/getLTokenBySToken",
        TokenKind::Cookie => {
            "https://passport-api.mihoyo.com/account/auth/api/getCookieAccountInfoBySToken"
        }
    };
    client
        .get(url)
        .header(
            "Cookie",
            format!(
                "stoken={stoken}; stuid={}; mid={}",
                identity.aid, identity.mid
            ),
        )
        .header("x-rpc-app_id", "bll8iq97cem8")
        .header("x-rpc-app_version", "2.95.1")
        .header("x-rpc-client_type", "2")
        .header("x-rpc-device_id", device)
        .header("x-rpc-game_biz", "bbs_cn")
        .header("x-rpc-sdk_version", "2.16.0")
        .header("Accept", "application/json")
        .header("User-Agent", "miHoYoBBS/2.95.1")
        .header(
            "DS",
            signature("JwYDpKvLj6MrMqqYU6jTKF17KNO2PXoS", Some("{}"), ""),
        )
}

pub(crate) async fn poll(pending: &Pending) -> Result<Poll, String> {
    let client = transport::client()?;
    let status: Envelope = transport::json(
        client
            .post("https://passport-api.mihoyo.com/account/ma-cn-passport/app/queryQRLoginStatus")
            .header("x-rpc-app_id", "ddxf5dufpuyo")
            .header("x-rpc-client_type", "3")
            .header("User-Agent", "HYPContainer/1.1.4.133")
            .header("Accept", "application/json")
            .header("x-rpc-device_id", &pending.device)
            .json(&serde_json::json!({"ticket":pending.ticket})),
    )
    .await
    .map_err(|error| format!("QR confirmation: {error}"))?;
    if matches!(status.retcode, -106 | -3501) {
        return Ok(Poll::Expired);
    }
    let data: QrStatus = status
        .decode()
        .map_err(|error| format!("QR confirmation: {error}"))?;
    let (tokens, identity, need_realperson) = match data {
        QrStatus::Created => return Ok(Poll::Waiting),
        QrStatus::Scanned => return Ok(Poll::Scanned),
        QrStatus::Confirmed {
            tokens,
            user_info,
            need_realperson,
        } => (tokens, user_info, need_realperson),
    };
    if need_realperson {
        return Err(
            "Complete identity verification in the official miHoYo app, then scan again."
                .to_owned(),
        );
    }
    let mut tokens = tokens.into_iter().filter(|token| token.token_type == 1);
    let token = tokens
        .next()
        .ok_or("miHoYo QR confirmation returned no SToken")?;
    if tokens.next().is_some() {
        return Err("miHoYo QR confirmation returned multiple STokens".to_owned());
    }
    if token.token.is_empty() || identity.aid.is_empty() || identity.mid.is_empty() {
        return Err("miHoYo QR confirmation returned incomplete credentials".to_owned());
    }
    let ltoken: LToken = request(passport(
        &client,
        TokenKind::LToken,
        &pending.device,
        &token.token,
        &identity,
    ))
    .await
    .map_err(|error| format!("LToken exchange: {error}"))?;
    if ltoken.ltoken.is_empty() {
        return Err("LToken exchange: miHoYo returned an empty token".to_owned());
    }
    Ok(Poll::Complete(Session::Mihoyo {
        account_id: identity.aid,
        stoken: token.token,
        mid: identity.mid,
        ltoken: ltoken.ltoken,
        device: pending.device.clone(),
        record: None,
    }))
}

fn authenticated(
    session: &Session,
    mut url: reqwest::Url,
    body: Option<String>,
) -> Result<reqwest::RequestBuilder, String> {
    let Session::Mihoyo {
        account_id,
        stoken,
        mid,
        ltoken,
        device,
        ..
    } = session
    else {
        return Err("miHoYo is not connected".to_owned());
    };
    let cookie = format!(
        "ltuid={account_id}; ltoken={ltoken}; stuid={account_id}; stoken={stoken}; mid={mid}"
    );
    let mut query: Vec<_> = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    query.sort();
    url.set_query(None);
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(&query);
    }
    let ds = signature(
        "xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs",
        Some(body.as_deref().unwrap_or("")),
        url.query().unwrap_or(""),
    );
    let client = transport::client()?;
    let builder = match body {
        Some(body) => client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body),
        None => client.get(url),
    };
    Ok(builder
        .header("Cookie", cookie)
        .header("DS", ds)
        .header("x-rpc-app_version", "2.71.1")
        .header("x-rpc-client_type", "5")
        .header("x-rpc-device_id", device)
        .header("Referer", "https://act.mihoyo.com/")
        .header("Origin", "https://act.mihoyo.com"))
}

#[derive(Deserialize)]
struct Roles {
    list: Vec<Role>,
}
#[derive(Deserialize)]
struct Role {
    game_uid: String,
    region: String,
    nickname: String,
    is_chosen: Option<bool>,
}

impl Roles {
    fn select(self, game: Game, region: &str) -> Result<Account, String> {
        let mut roles: Vec<_> = self
            .list
            .into_iter()
            .filter(|role| role.region == region)
            .collect();
        roles.sort_by_key(|role| !role.is_chosen.unwrap_or(false));
        let role = roles
            .into_iter()
            .next()
            .ok_or("No mainland official-server role is bound for this game")?;
        Ok(Account {
            game,
            uid: role.game_uid.clone(),
            role_id: role.game_uid,
            region: role.region,
            name: role.nickname,
        })
    }
}

#[derive(Deserialize)]
struct GenshinNotes {
    current_resin: u64,
    max_resin: u64,
    resin_recovery_time: String,
    finished_task_num: u64,
    total_task_num: u64,
    current_expedition_num: u64,
    max_expedition_num: u64,
    remain_resin_discount_num: u64,
    resin_discount_num_limit: u64,
}
#[derive(Deserialize)]
struct RailNotes {
    current_stamina: u64,
    max_stamina: u64,
    stamina_recover_time: i64,
    current_reserve_stamina: u64,
    current_train_score: u64,
    max_train_score: u64,
    accepted_epedition_num: u64,
    total_expedition_num: u64,
}
#[derive(Deserialize)]
struct Progress {
    current: u64,
    max: u64,
}
#[derive(Deserialize)]
struct Energy {
    progress: Progress,
    restore: i64,
}
#[derive(Deserialize)]
struct VideoStore {
    sale_state: String,
}
#[derive(Deserialize)]
struct ZzzNotes {
    energy: Energy,
    vitality: Progress,
    card_sign: String,
    vhs_sale: VideoStore,
}

fn notes_endpoint(game: Game) -> Result<&'static str, String> {
    Ok(match game {
        Game::Genshin => {
            "https://api-takumi-record.mihoyo.com/game_record/app/genshin/api/dailyNote"
        }
        Game::StarRail => "https://api-takumi-record.mihoyo.com/game_record/app/hkrpg/api/note",
        Game::Zzz => "https://api-takumi-record.mihoyo.com/event/game_record_zzz/api/zzz/note",
        _ => {
            return Err("Unsupported miHoYo game".to_owned());
        }
    })
}

pub(crate) async fn notes(
    record: &record::RecordSession,
    game: Game,
) -> Result<Notes, crate::NotesError> {
    let account = record.account(game).await?;
    let request = record.note_request(game, &account).await?;
    let mut result = Notes {
        account,
        sampled_at: transport::now(),
        meters: Vec::new(),
        tasks: Vec::new(),
    };
    let (response, headers): (Envelope, _) = transport::json_with_headers(request).await?;
    record
        .remember_verification_trace(game, response.retcode, &headers)
        .await;
    match game {
        Game::Genshin => {
            let data: GenshinNotes = response.note()?;
            let recovery: i64 = data
                .resin_recovery_time
                .parse()
                .map_err(|_| "Invalid resin recovery time")?;
            result.meters.push(Meter {
                label: "Original Resin".into(),
                current: data.current_resin,
                max: data.max_resin,
                full_at: Some(result.sampled_at + recovery),
            });
            result.tasks = [
                (
                    "Daily commissions",
                    data.finished_task_num,
                    data.total_task_num,
                ),
                (
                    "Expeditions",
                    data.current_expedition_num,
                    data.max_expedition_num,
                ),
                (
                    "Weekly discounts remaining",
                    data.remain_resin_discount_num,
                    data.resin_discount_num_limit,
                ),
            ]
            .into_iter()
            .map(|(label, current, max)| Task {
                progress: Some(crate::TaskProgress {
                    current,
                    total: max,
                }),
                label: label.into(),
                value: format!("{current} / {max}"),
            })
            .collect();
        }
        Game::StarRail => {
            let data: RailNotes = response.note()?;
            result.meters.push(Meter {
                label: "Trailblaze Power".into(),
                current: data.current_stamina,
                max: data.max_stamina,
                full_at: Some(result.sampled_at + data.stamina_recover_time),
            });
            result.tasks = vec![
                Task {
                    progress: None,
                    label: "Reserved power".into(),
                    value: data.current_reserve_stamina.to_string(),
                },
                Task {
                    // Each star represents a completed 100-point training milestone.
                    progress: Some(crate::TaskProgress {
                        current: data.current_train_score / 100,
                        total: data.max_train_score / 100,
                    }),
                    label: "Daily training".into(),
                    value: format!("{} / {}", data.current_train_score, data.max_train_score),
                },
                Task {
                    progress: Some(crate::TaskProgress {
                        current: data.accepted_epedition_num,
                        total: data.total_expedition_num,
                    }),
                    label: "Assignments".into(),
                    value: format!(
                        "{} / {}",
                        data.accepted_epedition_num, data.total_expedition_num
                    ),
                },
            ];
        }
        Game::Zzz => {
            let data: ZzzNotes = response.note()?;
            result.meters.push(Meter {
                label: "Battery Charge".into(),
                current: data.energy.progress.current,
                max: data.energy.progress.max,
                full_at: Some(result.sampled_at + data.energy.restore),
            });
            result.tasks = vec![
                Task {
                    progress: None,
                    label: "Daily engagement".into(),
                    value: format!("{} / {}", data.vitality.current, data.vitality.max),
                },
                Task {
                    progress: None,
                    label: "Scratch card".into(),
                    value: if data.card_sign == "CardSignDone" {
                        "Done"
                    } else {
                        "Not claimed"
                    }
                    .into(),
                },
                Task {
                    progress: None,
                    label: "Video store".into(),
                    value: match data.vhs_sale.sale_state.as_str() {
                        "SaleStateDone" => "Revenue ready",
                        "SaleStateDoing" => "Open",
                        "SaleStateNo" => "Closed",
                        _ => "Unknown",
                    }
                    .into(),
                },
            ];
        }
        _ => unreachable!(),
    }
    Ok(result)
}

#[derive(Deserialize)]
struct AuthKey {
    authkey: String,
    authkey_ver: u32,
    sign_type: u32,
}

impl AuthKey {
    fn append_query(&self, url: &mut reqwest::Url) -> Result<(), String> {
        // Decode percent escapes once, preserving literal '+' in base64 keys.
        // Url's query serializer then performs the single required encoding pass.
        let key = percent_encoding::percent_decode_str(&self.authkey)
            .decode_utf8()
            .map_err(|_| "Pull authorization contains an invalid key encoding")?;
        if key.is_empty()
            || key.len() > 64 * 1024
            || key.chars().any(char::is_control)
            || self.authkey_ver == 0
        {
            return Err("Pull authorization returned an invalid key or version".into());
        }
        url.query_pairs_mut()
            .append_pair("auth_appid", "webview_gacha")
            .append_pair("authkey", &key)
            .append_pair("authkey_ver", &self.authkey_ver.to_string())
            .append_pair("sign_type", &self.sign_type.to_string());
        Ok(())
    }
}

impl Envelope {
    fn gacha(self) -> Result<GachaPage, String> {
        if matches!(self.retcode, -100 | -101) {
            tracing::warn!(
                retcode = self.retcode,
                "miHoYo rejected pull-history authorization"
            );
            return Err("Pull-history authorization was rejected or has expired. Use Sync history to obtain a new key.".into());
        }
        self.decode()
    }
}
#[derive(Deserialize)]
struct GachaPage {
    list: Vec<GachaPull>,
}
#[derive(Deserialize)]
struct GachaPull {
    id: String,
    uid: String,
    gacha_type: String,
    item_id: Option<String>,
    name: String,
    rank_type: String,
    time: String,
}
#[derive(Deserialize)]
struct ZzzPage {
    gacha_item_list: Vec<ZzzPull>,
    has_more: bool,
}
#[derive(Deserialize)]
struct ZzzPull {
    id: String,
    item_id: u64,
    item_name: String,
    rarity: String,
    date: PullDate,
}
#[derive(Deserialize)]
struct PullDate {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

pub(crate) async fn pulls(
    record: &record::RecordSession,
    game: Game,
) -> Result<(Account, Vec<Pull>), String> {
    if !matches!(game, Game::Genshin | Game::Zzz) {
        return Err("This game does not use the AuthKey history flow.".into());
    }
    let account = record
        .account(game)
        .instrument(tracing::info_span!(
            "pull_history",
            game = game.key(),
            stage = "role_lookup"
        ))
        .await
        .map_err(|error| format!("Pull history role lookup: {error}"))?;
    let session = &record.session;
    let mut records = BTreeMap::new();
    let authkey = if game != Game::Zzz {
        let biz = "hk4e_cn";
        let body = serde_json::json!({"game_biz":biz,"game_uid":account.uid.parse::<u64>().map_err(|_| "Invalid game UID")?,"region":account.region,"auth_appid":"webview_gacha"}).to_string();
        let result: AuthKey = request(record.authkey_request(body)?)
            .instrument(tracing::info_span!(
                "pull_history",
                game = game.key(),
                stage = "authorization"
            ))
            .await
            .map_err(|error| format!("Pull history authorization: {error}"))?;
        Some(result)
    } else {
        None
    };
    let pools: &[(&str, &str)] = match game {
        Game::Genshin => &[
            ("100", "Beginner"),
            ("200", "Standard"),
            ("301", "Character event"),
            ("302", "Weapon event"),
            ("500", "Chronicled"),
        ],
        Game::Zzz => &[
            ("1", "Stable"),
            ("2", "Exclusive"),
            ("3", "W-Engine"),
            ("5", "Bangboo"),
            ("102", "Character rerun"),
            ("103", "W-Engine rerun"),
        ],
        _ => return Err("Unsupported miHoYo game".into()),
    };
    for (pool, pool_name) in pools {
        let mut cursor = "0".to_owned();
        let mut cursors = HashSet::new();
        let mut finished = false;
        for _ in 0..1000 {
            let endpoint = match game {
                Game::Genshin => {
                    "https://public-operation-hk4e.mihoyo.com/gacha_info/api/getGachaLog"
                }
                _ => {
                    "https://api-takumi-record.mihoyo.com/event/game_record_zzz/api/zzz/gacha_record"
                }
            };
            let mut url = reqwest::Url::parse(endpoint).map_err(|_| "Invalid history endpoint")?;
            url.query_pairs_mut()
                .append_pair("gacha_type", pool)
                .append_pair("end_id", &cursor);
            let (batch, more): (Vec<Pull>, bool) = if game == Game::Zzz {
                url.query_pairs_mut()
                    .append_pair("uid", &account.uid)
                    .append_pair("region", &account.region);
                let page: ZzzPage = request(authenticated(session, url, None)?).await?;
                let mut batch = Vec::new();
                for item in page.gacha_item_list {
                    let date = time::Date::from_calendar_date(
                        item.date.year,
                        item.date
                            .month
                            .try_into()
                            .map_err(|_| "Invalid pull month")?,
                        item.date.day,
                    )
                    .map_err(|_| "Invalid pull date")?;
                    let date = date
                        .with_hms(item.date.hour, item.date.minute, item.date.second)
                        .map_err(|_| "Invalid pull time")?;
                    let time = date
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]"
                        ))
                        .map_err(|_| "Invalid pull time")?;
                    let rarity = match item.rarity.as_str() {
                        "S" => 5,
                        "A" => 4,
                        "B" => 3,
                        _ => return Err("Unsupported ZZZ rarity".into()),
                    };
                    batch.push(Pull {
                        id: item.id,
                        pool: pool.to_string(),
                        pool_name: pool_name.to_string(),
                        item_id: Some(item.item_id.to_string()),
                        name: item.item_name,
                        rarity,
                        time,
                        is_free: None,
                        is_new: None,
                    });
                }
                (batch, page.has_more)
            } else {
                authkey
                    .as_ref()
                    .ok_or("Pull authorization is unavailable")?
                    .append_query(&mut url)?;
                url.query_pairs_mut()
                    .append_pair("lang", "zh-cn")
                    .append_pair("size", "20")
                    .append_pair("game_biz", "hk4e_cn");
                let response: Envelope = transport::json(transport::client()?.get(url))
                    .instrument(tracing::info_span!(
                        "pull_history",
                        game = game.key(),
                        stage = "download"
                    ))
                    .await
                    .map_err(|error| format!("Pull history download: {error}"))?;
                let page = response
                    .gacha()
                    .map_err(|error| format!("Pull history download: {error}"))?;
                let more = !page.list.is_empty();
                let mut batch = Vec::new();
                for item in page.list {
                    if item.uid != account.uid {
                        return Err("Pull history account does not match the signed-in role".into());
                    }
                    let category = if item.gacha_type == "400" {
                        "301".to_owned()
                    } else {
                        item.gacha_type
                    };
                    batch.push(Pull {
                        id: item.id,
                        pool: category,
                        pool_name: pool_name.to_string(),
                        item_id: item.item_id.filter(|id| !id.trim().is_empty()),
                        name: item.name,
                        rarity: item.rank_type.parse().map_err(|_| "Invalid pull rarity")?,
                        time: item.time,
                        is_free: None,
                        is_new: None,
                    });
                }
                (batch, more)
            };
            if more && batch.is_empty() {
                return Err("History pagination returned an empty continuation".into());
            }
            if let Some(last) = batch.last() {
                cursor = last.id.clone();
            }
            for item in batch {
                let existing = records.insert(item.id.clone(), item.clone());
                if existing.is_some_and(|existing| existing != item) {
                    return Err("Conflicting pull records returned by provider".into());
                }
            }
            if !more {
                finished = true;
                break;
            }
            if !cursors.insert(cursor.clone()) {
                return Err("Repeated history cursor; archive was not changed".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        if !finished {
            return Err("History exceeds the sync page limit; archive was not changed".into());
        }
    }
    Ok((account, records.into_values().collect()))
}

pub(crate) mod rail_gacha;
#[cfg(test)]
#[path = "../../tests/unit/mihoyo/mod.rs"]
mod tests;
