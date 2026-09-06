use super::{Envelope, record::RecordSession};
use crate::{Account, transport};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub pools: Vec<Pool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub total: u64,
    pub since_high_rarity: Option<u64>,
    pub five_stars: Vec<FiveStar>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveStar {
    pub id: String,
    pub item_id: u64,
    pub name: String,
    pub pulls: u64,
    pub is_up: bool,
}

#[derive(Deserialize)]
struct Stats {
    cards: Vec<Card>,
}
#[derive(Deserialize)]
struct Card {
    total_count: u64,
}
#[derive(Deserialize)]
struct Page {
    list: Vec<Entry>,
    has_more: bool,
    version_id: String,
    next_max_id: String,
}
#[derive(Deserialize)]
struct Entry {
    id: String,
    gacha_count: u64,
    is_up: bool,
    item: Option<Item>,
}
#[derive(Deserialize)]
struct Item {
    item_id: u64,
    name: String,
    rarity: u8,
}

pub(crate) async fn read(record: &RecordSession) -> Result<(Account, Report), String> {
    let account = record.account(crate::Game::StarRail).await?;
    let activity = Activity::login(record, account.clone())
        .await
        .map_err(|error| format!("Star Rail pull login: {error}"))?;
    let mut pools = Vec::new();
    for (id, name) in [
        ("11", "Character event"),
        ("12", "Light cone event"),
        ("21", "Character collaboration"),
        ("22", "Light cone collaboration"),
        ("1", "Stellar"),
        ("2", "Departure"),
    ] {
        let stats: Stats = activity.get("pool_stat", &[("gacha_type", id)]).await?;
        let total = stats
            .cards
            .iter()
            .try_fold(0u64, |total, card| total.checked_add(card.total_count))
            .ok_or("Invalid Star Rail pull total")?;
        let mut pool = Pool {
            id: id.into(),
            name: name.into(),
            total,
            since_high_rarity: None,
            five_stars: Vec::new(),
        };
        let mut cursor: Option<(String, String)> = None;
        let mut cursors = HashSet::new();
        let mut ids = HashSet::new();
        for index in 0..50 {
            let mut query = vec![("gacha_type", id)];
            if let Some((version, max_id)) = &cursor {
                query.extend([
                    ("version_id", version.as_str()),
                    ("max_id", max_id.as_str()),
                ]);
            }
            let page: Page = activity.get("five_star_list", &query).await?;
            for entry in page.list {
                if let Some(item) = entry.item {
                    if entry.id.is_empty() || item.name.trim().is_empty() || item.rarity != 5 {
                        return Err("Star Rail returned an invalid five-star record.".into());
                    }
                    if ids.insert(entry.id.clone()) {
                        pool.five_stars.push(FiveStar {
                            id: entry.id,
                            item_id: item.item_id,
                            name: item.name,
                            pulls: entry.gacha_count,
                            is_up: entry.is_up,
                        });
                    }
                } else if index == 0 && pool.since_high_rarity.is_none() {
                    pool.since_high_rarity = Some(entry.gacha_count);
                }
            }
            if !page.has_more {
                break;
            }
            let next = (page.version_id, page.next_max_id);
            if index == 49
                || next.0.is_empty()
                || next.1.is_empty()
                || !cursors.insert(next.clone())
            {
                return Err(
                    "Star Rail pull pagination did not finish; the previous archive is unchanged."
                        .into(),
                );
            }
            cursor = Some(next);
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        pools.push(pool);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    Ok((account, Report { pools }))
}

const LOGIN: &str = "https://api-takumi.mihoyo.com/common/badge/v1/login/account";
const API: &str = "https://act-api-takumi.mihoyo.com/event/rpg_gacha_record";
const REFERER: &str = "https://act.mihoyo.com/sr/event/gt-aio/gacha-records/index.html";

struct Activity {
    cookie: String,
    device: String,
    account: Account,
}

impl Activity {
    async fn login(record: &RecordSession, account: Account) -> Result<Self, String> {
        let mut cookie = record
            .cookies
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        let request = transport::client()?.post(LOGIN)
            .header("Cookie", &cookie).header("Origin", "https://act.mihoyo.com")
            .header("Referer", REFERER).header("User-Agent", super::record::USER_AGENT)
            .json(&serde_json::json!({"uid":account.uid,"region":account.region,"game_biz":"hkrpg_cn","lang":"zh-cn"}));
        let (response, headers): (Envelope, _) = transport::json_with_headers(request).await?;
        response.decode::<serde_json::Value>()?;
        let token = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| value.split(';').next()?.split_once('='))
            .find(|(name, _)| *name == "e_hkrpg_token")
            .map(|(_, value)| value)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 8192
                    && value.bytes().all(|b| b.is_ascii_graphic() && b != b';')
            })
            .ok_or("Star Rail activity login did not return an authorization cookie.")?;
        cookie.push_str("; e_hkrpg_token=");
        cookie.push_str(token);
        Ok(Self {
            cookie,
            device: record.device.clone(),
            account,
        })
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<T, String> {
        let mut url = reqwest::Url::parse(&format!("{API}/{endpoint}"))
            .map_err(|_| "Invalid Star Rail activity endpoint")?;
        url.query_pairs_mut()
            .append_pair("badge_uid", &self.account.uid)
            .append_pair("badge_region", &self.account.region)
            .append_pair("game_biz", "hkrpg_cn")
            .append_pair("uid", &self.account.uid)
            .append_pair("region", &self.account.region)
            .extend_pairs(params.iter().copied());
        let response: Envelope = transport::json(
            transport::client()?
                .get(url)
                .header("Cookie", &self.cookie)
                .header("Origin", "https://act.mihoyo.com")
                .header("Referer", REFERER)
                .header("User-Agent", super::record::USER_AGENT)
                .header("x-rpc-device_id", &self.device)
                .header("x-rpc-jump_source", "wechatmp")
                .header("x-rpc-platform", "4"),
        )
        .await?;
        response.decode()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/mihoyo/rail_gacha.rs"]
mod tests;
