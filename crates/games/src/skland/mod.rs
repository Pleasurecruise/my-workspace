use crate::{
    Account, Game, Meter, Notes, Pull, Task,
    login::{Pending, Poll},
    transport,
};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use serde::{Deserialize, de::DeserializeOwned};
use sha2_legacy::Sha256;
use std::collections::{BTreeMap, HashSet};
use vesper_credentials::games::Session;

#[derive(Deserialize)]
struct Envelope {
    code: Option<i64>,
    status: Option<i64>,
    data: Option<serde_json::Value>,
}

async fn request<T: DeserializeOwned>(builder: reqwest::RequestBuilder) -> Result<T, String> {
    let result: Envelope = transport::json(builder).await?;
    result.decode()
}

impl Envelope {
    fn decode<T: DeserializeOwned>(self) -> Result<T, String> {
        match self.code.or(self.status) {
            Some(0) => {
                serde_json::from_value(self.data.ok_or("Skland returned no data")?).map_err(|_| {
                    "Skland returned an unsupported response. Please update Vesper.".to_owned()
                })
            }
            Some(10000 | 10002) => Err("Skland login expired. Scan again in Settings.".into()),
            Some(code) => Err(format!("Skland request failed (code {code})")),
            None => Err("Invalid Skland response".into()),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Scan {
    scan_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanCode {
    scan_code: String,
}
enum ScanState {
    Waiting,
    Scanned,
    Expired,
    Confirmed(ScanCode),
}

impl Envelope {
    fn scan(self) -> Result<ScanState, String> {
        match self.status.or(self.code) {
            Some(0) => self.decode().map(ScanState::Confirmed),
            Some(100) => Ok(ScanState::Waiting),
            Some(101) => Ok(ScanState::Scanned),
            Some(102) => Ok(ScanState::Expired),
            Some(code) => Err(format!("Skland QR login failed (code {code})")),
            None => Err("Invalid Skland QR response".into()),
        }
    }
}
#[derive(Deserialize)]
struct Token {
    token: String,
}
#[derive(Deserialize)]
struct Grant {
    code: Option<String>,
    token: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cred {
    cred: String,
    token: String,
    user_id: String,
}

pub(crate) async fn begin() -> Result<(Pending, String), String> {
    let scan: Scan = request(
        transport::client()?
            .post("https://as.hypergryph.com/general/v1/gen_scan/login")
            .json(&serde_json::json!({"appCode":"4ca99fa6b56cc2ba"})),
    )
    .await?;
    let mut url =
        reqwest::Url::parse("hypergryph://scan_login").map_err(|_| "Invalid Skland QR endpoint")?;
    url.query_pairs_mut().append_pair("scanId", &scan.scan_id);
    Ok((
        Pending {
            ticket: scan.scan_id,
            device: String::new(),
        },
        url.to_string(),
    ))
}

pub(crate) async fn poll(pending: &Pending) -> Result<Poll, String> {
    let client = transport::client()?;
    let response: Envelope = transport::json(
        client
            .get("https://as.hypergryph.com/general/v1/scan_status")
            .query(&[("scanId", &pending.ticket)]),
    )
    .await?;
    let scan = match response.scan()? {
        ScanState::Waiting => return Ok(Poll::Waiting),
        ScanState::Scanned => return Ok(Poll::Scanned),
        ScanState::Expired => return Ok(Poll::Expired),
        ScanState::Confirmed(scan) => scan,
    };
    if scan.scan_code.is_empty() {
        return Ok(Poll::Waiting);
    }
    let token: Token = request(
        client
            .post("https://as.hypergryph.com/user/auth/v1/token_by_scan_code")
            .json(&serde_json::json!({"scanCode":scan.scan_code})),
    )
    .await?;
    let grant: Grant = request(
        client
            .post("https://as.hypergryph.com/user/oauth2/v2/grant")
            .json(&serde_json::json!({"appCode":"4ca99fa6b56cc2ba","token":token.token,"type":0})),
    )
    .await?;
    let code = grant.code.ok_or("Missing Skland authorization code")?;
    let cred: Cred = request(
        client
            .post("https://zonai.skland.com/api/v1/user/auth/generate_cred_by_code")
            .json(&serde_json::json!({"code":code,"kind":1})),
    )
    .await?;
    Ok(Poll::Complete(Session::Skland {
        token: token.token,
        cred: cred.cred,
        sign_token: cred.token,
        user_id: cred.user_id,
    }))
}

struct Client<'a> {
    client: reqwest::Client,
    cred: &'a str,
    token: String,
}
impl<'a> Client<'a> {
    async fn new(session: &'a Session) -> Result<Self, String> {
        let Session::Skland { cred, .. } = session else {
            return Err("Skland is not connected".into());
        };
        let client = transport::client()?;
        let token: Token = request(
            client
                .get("https://zonai.skland.com/api/v1/auth/refresh")
                .header("cred", cred),
        )
        .await?;
        Ok(Self {
            client,
            cred,
            token: token.token,
        })
    }
    async fn get<T: DeserializeOwned>(&self, url: reqwest::Url) -> Result<T, String> {
        let timestamp = (transport::now() - 1).to_string();
        let headers = format!(
            "{{\"platform\":\"\",\"timestamp\":\"{timestamp}\",\"dId\":\"\",\"vName\":\"\"}}"
        );
        let input = format!(
            "{}{}{timestamp}{headers}",
            url.path(),
            url.query().unwrap_or("")
        );
        let mut hmac = Hmac::<Sha256>::new_from_slice(self.token.as_bytes())
            .map_err(|_| "Invalid Skland signing key")?;
        hmac.update(input.as_bytes());
        let digest = hex::encode(hmac.finalize().into_bytes());
        let signature = format!("{:x}", Md5::digest(digest.as_bytes()));
        request(self.client.get(url)
            .header("cred", self.cred)
            .header("sign", signature)
            .header("platform", "")
            .header("timestamp", timestamp)
            .header("dId", "")
            .header("vName", "")
            .header("User-Agent", "Skland/1.32.1 (com.hypergryph.skland; build:103201004; Android 33; ) Okhttp/4.11.0"))
            .await
    }
    async fn account(&self, game: Game) -> Result<Account, String> {
        let url = reqwest::Url::parse("https://zonai.skland.com/api/v1/game/player/binding")
            .map_err(|_| "Invalid Skland binding endpoint")?;
        let bindings: Bindings = self.get(url).await?;
        let code = match game {
            Game::Arknights => "arknights",
            Game::Endfield => "endfield",
            _ => return Err("Unsupported Skland game".into()),
        };
        let binding = bindings
            .list
            .into_iter()
            .find(|binding| binding.app_code == code)
            .ok_or("No bound role for this game")?;
        let mut accounts: Vec<_> = binding
            .binding_list
            .into_iter()
            .filter(|role| role.is_official && !role.is_delete)
            .collect();
        accounts.sort_by_key(|role| !role.is_default);
        let binding = accounts
            .into_iter()
            .next()
            .ok_or("No mainland official-server account is bound")?;
        if game == Game::Arknights {
            return Ok(Account {
                game,
                uid: binding.uid.clone(),
                role_id: binding.uid,
                region: binding.channel_master_id,
                name: binding.nick_name,
            });
        }
        let mut roles = binding.roles;
        roles.sort_by_key(|role| !role.is_default);
        let role = roles
            .into_iter()
            .find(|role| !role.is_banned)
            .ok_or("No Endfield role is bound")?;
        Ok(Account {
            game,
            uid: binding.uid,
            role_id: role.role_id,
            region: role.server_id,
            name: role.nickname,
        })
    }
}

#[derive(Deserialize)]
struct Bindings {
    list: Vec<Binding>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Binding {
    app_code: String,
    binding_list: Vec<BoundAccount>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoundAccount {
    uid: String,
    is_official: bool,
    is_default: bool,
    is_delete: bool,
    channel_master_id: String,
    nick_name: String,
    #[serde(default)]
    roles: Vec<Role>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Role {
    server_id: String,
    role_id: String,
    nickname: String,
    is_default: bool,
    is_banned: bool,
}
#[derive(Deserialize)]
struct Count {
    current: u64,
    total: u64,
}
#[derive(Deserialize)]
struct Routine {
    daily: Count,
    weekly: Count,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ap {
    current: u64,
    max: u64,
    complete_recovery_time: i64,
}
#[derive(Deserialize)]
struct Status {
    ap: Ap,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recruit {
    state: i64,
    finish_ts: i64,
}
#[derive(Deserialize)]
struct ArkCard {
    status: Status,
    routine: Routine,
    recruit: Vec<Recruit>,
}
#[derive(Deserialize)]
struct Card {
    detail: EndfieldCard,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndfieldCard {
    dungeon: Dungeon,
    daily_mission: Daily,
    weekly_mission: Weekly,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Dungeon {
    cur_stamina: String,
    max_ts: String,
    max_stamina: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Daily {
    daily_activation: u64,
    max_daily_activation: u64,
}
#[derive(Deserialize)]
struct Weekly {
    score: u64,
    total: u64,
}

pub(crate) async fn notes(session: &Session, game: Game) -> Result<Notes, String> {
    let client = Client::new(session).await?;
    let account = client.account(game).await?;
    let sampled_at = transport::now();
    let (meters, tasks) = if game == Game::Arknights {
        let mut url = reqwest::Url::parse("https://zonai.skland.com/api/v1/game/player/info")
            .map_err(|_| "Invalid Skland endpoint")?;
        url.query_pairs_mut().append_pair("uid", &account.uid);
        let card: ArkCard = client.get(url).await?;
        let ap = card.status.ap;
        let current = if ap.current >= ap.max {
            ap.current
        } else if ap.complete_recovery_time > 0 {
            ap.max.saturating_sub(
                u64::try_from((ap.complete_recovery_time - sampled_at).max(0))
                    .map_err(|_| "Invalid sanity recovery time")?
                    .div_ceil(360),
            )
        } else {
            ap.current
        };
        let ready = card
            .recruit
            .iter()
            .filter(|recruit| {
                if recruit.state == 1 {
                    return true;
                }
                recruit.finish_ts > 0 && recruit.finish_ts <= sampled_at
            })
            .count();
        (
            vec![Meter {
                label: "Sanity".into(),
                current,
                max: ap.max,
                full_at: (ap.complete_recovery_time > 0).then_some(ap.complete_recovery_time),
            }],
            vec![
                Task {
                    progress: None,
                    label: "Daily missions".into(),
                    value: format!(
                        "{} / {}",
                        card.routine.daily.current, card.routine.daily.total
                    ),
                },
                Task {
                    progress: None,
                    label: "Weekly missions".into(),
                    value: format!(
                        "{} / {}",
                        card.routine.weekly.current, card.routine.weekly.total
                    ),
                },
                Task {
                    progress: None,
                    label: "Recruitments ready".into(),
                    value: ready.to_string(),
                },
            ],
        )
    } else {
        let Session::Skland { user_id, .. } = session else {
            return Err("Skland is not connected".into());
        };
        let mut url =
            reqwest::Url::parse("https://zonai.skland.com/web/v1/game/endfield/card/detail")
                .map_err(|_| "Invalid Endfield endpoint")?;
        url.query_pairs_mut()
            .append_pair("roleId", &account.role_id)
            .append_pair("serverId", &account.region)
            .append_pair("userId", user_id);
        let card: Card = client.get(url).await?;
        let data = card.detail;
        let current = data
            .dungeon
            .cur_stamina
            .parse()
            .map_err(|_| "Invalid Endfield stamina")?;
        let max = data
            .dungeon
            .max_stamina
            .parse()
            .map_err(|_| "Invalid Endfield stamina maximum")?;
        let full: i64 = data
            .dungeon
            .max_ts
            .parse()
            .map_err(|_| "Invalid Endfield recovery time")?;
        (
            vec![Meter {
                label: "Sanity".into(),
                current,
                max,
                full_at: (full > 0).then_some(full),
            }],
            vec![
                Task {
                    progress: None,
                    label: "Daily missions".into(),
                    value: format!(
                        "{} / {}",
                        data.daily_mission.daily_activation,
                        data.daily_mission.max_daily_activation
                    ),
                },
                Task {
                    progress: None,
                    label: "Weekly missions".into(),
                    value: format!(
                        "{} / {}",
                        data.weekly_mission.score, data.weekly_mission.total
                    ),
                },
            ],
        )
    };
    Ok(Notes {
        account,
        sampled_at,
        meters,
        tasks,
    })
}

#[derive(Deserialize)]
struct RoleToken {
    token: String,
}
#[derive(Deserialize)]
struct Category {
    id: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page<T> {
    list: Vec<T>,
    has_more: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArkPull {
    pool_id: String,
    pool_name: String,
    char_id: String,
    char_name: String,
    rarity: u8,
    gacha_ts: String,
    pos: u32,
    is_new: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EfPull {
    pool_id: Option<String>,
    pool_name: Option<String>,
    char_id: Option<String>,
    char_name: Option<String>,
    weapon_id: Option<String>,
    weapon_name: Option<String>,
    rarity: Option<u8>,
    gacha_ts: String,
    seq_id: String,
    kind: Option<String>,
    is_free: Option<bool>,
    is_new: Option<bool>,
}

pub(crate) async fn pulls(session: &Session, game: Game) -> Result<(Account, Vec<Pull>), String> {
    let client = Client::new(session).await?;
    let account = client.account(game).await?;
    let Session::Skland { token, .. } = session else {
        return Err("Skland is not connected".into());
    };
    let grant: Grant = request(
        client
            .client
            .post("https://as.hypergryph.com/user/oauth2/v2/grant")
            .json(&serde_json::json!({"appCode":"be36d44aa36bfb5b","token":token,"type":1})),
    )
    .await?;
    let granted = grant.token.ok_or("Missing game account authorization")?;
    let role: RoleToken = request(client.client
        .post("https://binding-api-account-prod.hypergryph.com/account/binding/v1/u8_token_by_uid")
        .json(&serde_json::json!({"uid":account.uid,"token":granted})))
        .await?;
    let mut cookie = String::new();
    let categories = if game == Game::Arknights {
        let response = client
            .client
            .post("https://ak.hypergryph.com/user/api/role/login")
            .json(&serde_json::json!({"token":role.token}))
            .send()
            .await
            .map_err(|_| "Could not authorize Arknights history")?;
        if !response.status().is_success() {
            return Err("Arknights history authorization failed".into());
        }
        for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
            let Ok(value) = value.to_str() else {
                continue;
            };
            if let Some(value) = value
                .split(';')
                .next()
                .filter(|value| value.starts_with("ak-user-center="))
            {
                cookie = value.to_owned();
            }
        }
        if cookie.is_empty() {
            return Err("Arknights history login did not return a session".into());
        }
        request::<Vec<Category>>(
            client
                .client
                .get("https://ak.hypergryph.com/user/api/inquiry/gacha/cate")
                .query(&[("uid", &account.uid)])
                .header("X-Account-Token", token)
                .header("X-Role-Token", &role.token)
                .header("Cookie", &cookie),
        )
        .await?
    } else {
        [
            ("E_CharacterGachaPoolType_Standard", "Standard"),
            ("E_CharacterGachaPoolType_Special", "Special"),
            ("E_CharacterGachaPoolType_Beginner", "Beginner"),
            ("E_CharacterGachaPoolType_Joint", "Joint"),
            ("weapon", "Weapon"),
        ]
        .into_iter()
        .map(|(id, name)| Category {
            id: id.into(),
            name: name.into(),
        })
        .collect()
    };
    let mut records = BTreeMap::new();
    for category in categories {
        let mut cursor = String::new();
        let mut pos = 0;
        let mut seen = HashSet::new();
        let mut finished = false;
        for _ in 0..1000 {
            let mut batch = Vec::new();
            let more;
            if game == Game::Arknights {
                let mut builder = client
                    .client
                    .get("https://ak.hypergryph.com/user/api/inquiry/gacha/history")
                    .query(&[
                        ("uid", account.uid.as_str()),
                        ("category", category.id.as_str()),
                        ("size", "100"),
                    ])
                    .header("X-Account-Token", token)
                    .header("X-Role-Token", &role.token)
                    .header("Cookie", &cookie);
                if !cursor.is_empty() {
                    builder =
                        builder.query(&[("gachaTs", cursor.clone()), ("pos", pos.to_string())]);
                }
                let page: Page<ArkPull> = request(builder).await?;
                more = page.has_more;
                if let Some(last) = page.list.last() {
                    cursor = last.gacha_ts.clone();
                    pos = last.pos;
                }
                for item in page.list {
                    let timestamp: i64 = item
                        .gacha_ts
                        .parse()
                        .map_err(|_| "Invalid pull timestamp")?;
                    let date = time::OffsetDateTime::from_unix_timestamp(timestamp / 1000)
                        .map_err(|_| "Invalid pull timestamp")?
                        .to_offset(time::macros::offset!(+8));
                    let time = date
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]"
                        ))
                        .map_err(|_| "Invalid pull timestamp")?;
                    batch.push(Pull {
                        id: format!("{}-{}", item.gacha_ts, item.pos),
                        pool: item.pool_id,
                        pool_name: item.pool_name,
                        item_id: Some(item.char_id),
                        name: item.char_name,
                        rarity: item
                            .rarity
                            .checked_add(1)
                            .ok_or("Invalid Arknights rarity")?,
                        time,
                        is_free: None,
                        is_new: Some(item.is_new),
                    });
                }
            } else {
                let weapon = category.id == "weapon";
                let endpoint = if weapon {
                    "https://ef-webview.hypergryph.com/api/record/weapon"
                } else {
                    "https://ef-webview.hypergryph.com/api/record/char"
                };
                let mut builder = client.client.get(endpoint).query(&[
                    ("token", role.token.as_str()),
                    ("server_id", account.region.as_str()),
                    ("lang", "zh-cn"),
                ]);
                if !weapon {
                    builder = builder.query(&[("pool_type", &category.id)]);
                }
                if !cursor.is_empty() {
                    builder = builder.query(&[("seq_id", &cursor)]);
                }
                let page: Page<EfPull> = request(builder).await?;
                more = page.has_more;
                if let Some(last) = page.list.last() {
                    cursor = last.seq_id.clone();
                }
                for item in page.list {
                    if !weapon {
                        match item.kind.as_deref() {
                            Some("draw") => {}
                            Some(_) => continue,
                            None => return Err("Missing Endfield history event type".into()),
                        }
                    }
                    let timestamp: i64 = item
                        .gacha_ts
                        .parse()
                        .map_err(|_| "Invalid pull timestamp")?;
                    let date = time::OffsetDateTime::from_unix_timestamp(timestamp / 1000)
                        .map_err(|_| "Invalid pull timestamp")?
                        .to_offset(time::macros::offset!(+8));
                    let time = date
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]"
                        ))
                        .map_err(|_| "Invalid pull timestamp")?;
                    let (item_id, name) = if weapon {
                        (item.weapon_id, item.weapon_name)
                    } else {
                        (item.char_id, item.char_name)
                    };
                    batch.push(Pull {
                        id: format!("{}-{}", if weapon { "weapon" } else { "char" }, item.seq_id),
                        pool: item.pool_id.ok_or("Missing pool ID")?,
                        pool_name: item.pool_name.ok_or("Missing pool name")?,
                        item_id: Some(item_id.ok_or("Missing pull item ID")?),
                        name: name.ok_or("Missing pull name")?,
                        rarity: item.rarity.ok_or("Missing pull rarity")?,
                        time,
                        is_free: if weapon {
                            Some(false)
                        } else {
                            Some(item.is_free.ok_or("Missing free-pull flag")?)
                        },
                        is_new: Some(item.is_new.ok_or("Missing new-item flag")?),
                    });
                }
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
            if cursor.is_empty() || !seen.insert((cursor.clone(), pos)) {
                return Err("History pagination did not advance; archive was not changed".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        if !finished {
            return Err(format!(
                "{} history exceeds the sync page limit",
                category.name
            ));
        }
    }
    Ok((account, records.into_values().collect()))
}

#[cfg(test)]
#[path = "../../tests/unit/skland/mod.rs"]
mod tests;
