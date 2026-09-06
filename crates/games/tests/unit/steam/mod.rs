use super::*;

#[test]
fn private_activity_remains_absent_instead_of_zero() {
    let recent: Response<Recent> = serde_json::from_str(r#"{"response":{}}"#).unwrap();
    assert_eq!(recent.response.total_count, None);
    assert!(recent.response.games.is_empty());
    let owned: Response<Owned> = serde_json::from_str(r#"{"response":{}}"#).unwrap();
    assert_eq!(owned.response.game_count, None);
    let empty: Response<Recent> =
        serde_json::from_str(r#"{"response":{"total_count":0}}"#).unwrap();
    assert_eq!(empty.response.total_count, Some(0));
}

#[test]
fn activity_projection_preserves_minutes_with_camel_case_fields() {
    let game: RecentGame = serde_json::from_str(
        r#"{"appid":10,"name":"Game","playtime_forever":123,"playtime_2weeks":45}"#,
    )
    .unwrap();
    let output = serde_json::to_value(game).unwrap();
    assert_eq!(output["appId"], 10);
    assert_eq!(output["playtimeForever"], 123);
    assert_eq!(output["playtime2weeks"], 45);
}

#[test]
fn totals_include_games_beyond_the_display_limit() {
    let player = serde_json::from_str(
        r#"{"personaname":"Player","profileurl":"https://steamcommunity.com/id/example"}"#,
    )
    .unwrap();
    let games: Vec<RecentGame> = (1..=7)
        .map(|app_id| RecentGame {
            app_id,
            name: format!("Game {app_id}"),
            playtime_forever: app_id * 100,
            playtime_2weeks: Some(app_id * 10),
        })
        .collect();
    let result = project(
        player,
        Recent {
            total_count: Some(7),
            games: games.clone(),
        },
        Owned {
            game_count: Some(7),
            games,
        },
    );
    assert_eq!(result.recent_minutes, Some(280));
    assert_eq!(result.total_minutes, Some(2800));
    assert_eq!(result.played_games, Some(7));
    assert_eq!(result.recent.len(), 5);
    assert_eq!(result.most_played.len(), 5);
    assert_eq!(result.recent[0].app_id, 7);
    assert_eq!(result.most_played[0].app_id, 7);
    let output = serde_json::to_value(result).unwrap();
    assert!(output.get("apiKey").is_none());
    assert!(output.get("steamId").is_none());
}

#[test]
fn hidden_library_and_activity_do_not_claim_zero_playtime() {
    let player = serde_json::from_str(
        r#"{"personaname":"Player","profileurl":"https://steamcommunity.com/id/example"}"#,
    )
    .unwrap();
    let result = project(
        player,
        serde_json::from_str("{}").unwrap(),
        serde_json::from_str("{}").unwrap(),
    );
    assert_eq!(result.recent_minutes, None);
    assert_eq!(result.total_minutes, None);
    assert_eq!(result.played_games, None);
}
