use crate::transport;
use serde::{Deserialize, Serialize};
use vesper_credentials::games::Session;

#[derive(Deserialize)]
struct Response<T> {
    response: T,
}
#[derive(Deserialize)]
struct Players {
    players: Vec<Player>,
}
#[derive(Deserialize)]
struct Player {
    personaname: String,
    profileurl: String,
    personastate: Option<u8>,
    gameextrainfo: Option<String>,
}
#[derive(Deserialize)]
struct Recent {
    total_count: Option<u64>,
    #[serde(default)]
    games: Vec<RecentGame>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct RecentGame {
    #[serde(rename(deserialize = "appid"))]
    pub app_id: u64,
    pub name: String,
    pub playtime_forever: u64,
    pub playtime_2weeks: Option<u64>,
}
#[derive(Deserialize)]
struct Owned {
    game_count: Option<u64>,
    #[serde(default)]
    games: Vec<RecentGame>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub name: String,
    pub profile_url: String,
    pub state: Option<u8>,
    pub playing: Option<String>,
    pub owned_games: Option<u64>,
    pub recent_count: Option<u64>,
    pub recent_minutes: Option<u64>,
    pub total_minutes: Option<u64>,
    pub played_games: Option<u64>,
    pub most_played: Vec<RecentGame>,
    pub recent: Vec<RecentGame>,
    pub sampled_at: i64,
}

pub(crate) async fn read(session: &Session) -> Result<Snapshot, String> {
    let Session::Steam { api_key, steam_id } = session else {
        return Err("Steam is not connected".to_owned());
    };
    if steam_id.len() != 17
        || !steam_id.bytes().all(|c| c.is_ascii_digit())
        || api_key.trim().is_empty()
    {
        return Err("Enter a Steam API key and a 17-digit SteamID64".to_owned());
    }
    let client = transport::client()?;
    let (players, recent, owned) = tokio::try_join!(
        transport::json::<Response<Players>>(
            client
                .get("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/")
                .header("x-webapi-key", api_key)
                .query(&[("steamids", steam_id.as_str())])
        ),
        transport::json::<Response<Recent>>(
            client
                .get("https://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v1/")
                .header("x-webapi-key", api_key)
                .query(&[("steamid", steam_id.as_str())])
        ),
        transport::json::<Response<Owned>>(
            client
                .get("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/")
                .header("x-webapi-key", api_key)
                .query(&[
                    ("steamid", steam_id.as_str()),
                    ("include_appinfo", "true"),
                    ("include_played_free_games", "true")
                ])
        )
    )?;
    let player = players
        .response
        .players
        .into_iter()
        .next()
        .ok_or("Steam profile was not found")?;
    Ok(project(player, recent.response, owned.response))
}

fn project(player: Player, mut recent: Recent, mut owned: Owned) -> Snapshot {
    let recent_minutes = recent
        .total_count
        .and_then(|_| recent.games.iter().map(|game| game.playtime_2weeks).sum());
    let total_minutes = owned
        .game_count
        .map(|_| owned.games.iter().map(|game| game.playtime_forever).sum());
    let played_games = owned.game_count.map(|_| {
        owned
            .games
            .iter()
            .filter(|game| game.playtime_forever > 0)
            .count() as u64
    });
    owned.games.retain(|game| game.playtime_forever > 0);
    owned
        .games
        .sort_by_key(|game| (std::cmp::Reverse(game.playtime_forever), game.app_id));
    owned.games.truncate(5);
    recent
        .games
        .sort_by_key(|game| (std::cmp::Reverse(game.playtime_2weeks), game.app_id));
    recent.games.truncate(5);
    Snapshot {
        name: player.personaname,
        profile_url: player.profileurl,
        state: player.personastate,
        playing: player.gameextrainfo,
        owned_games: owned.game_count,
        recent_count: recent.total_count,
        recent: recent.games,
        recent_minutes,
        total_minutes,
        played_games,
        most_played: owned.games,
        sampled_at: transport::now(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/steam/mod.rs"]
mod tests;
