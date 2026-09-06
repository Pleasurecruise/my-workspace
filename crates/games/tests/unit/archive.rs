use super::*;

fn account(uid: &str) -> Account {
    Account {
        game: Game::Genshin,
        uid: uid.into(),
        region: "cn_gf01".into(),
        name: "Traveler".into(),
        role_id: uid.into(),
    }
}
fn pull(id: &str, rarity: u8) -> Pull {
    Pull {
        id: id.into(),
        pool: "301".into(),
        pool_name: "Character".into(),
        item_id: Some("1".into()),
        name: "Item".into(),
        rarity,
        time: "2026-09-06 12:00:00".into(),
        is_free: None,
        is_new: None,
    }
}

#[test]
fn missing_item() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite");
    let mut item = pull("1", 3);
    item.item_id = None;
    let saved = merge(&path, &account("100"), &[item.clone()]).unwrap();
    assert_eq!(saved.total, 1);
    assert_eq!(saved.recent[0].item_id, None);
    let json = serde_json::to_value(&saved).unwrap();
    assert!(json["recent"][0]["itemId"].is_null());
    assert_eq!(saved.pools[0].rarities, [0, 0, 1, 0, 0, 0]);
    item.item_id = Some("known".into());
    let enriched = merge(&path, &account("100"), &[item.clone()]).unwrap();
    assert_eq!(enriched.added, 0);
    assert_eq!(enriched.recent[0].item_id.as_deref(), Some("known"));
    item.item_id = None;
    let retained = merge(&path, &account("100"), &[item.clone()]).unwrap();
    assert_eq!(retained.recent[0].item_id.as_deref(), Some("known"));
    item.name = "Different item".into();
    assert!(merge(&path, &account("100"), &[item]).is_err());
    assert_eq!(summary(&path, Game::Genshin, Some("100")).unwrap().total, 1);
}

#[test]
fn preserves_history_across_windows_accounts_and_restarts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite");
    let first = merge(
        &path,
        &account("100"),
        &[pull("1", 5), pull("2", 3), pull("3", 5)],
    )
    .unwrap();
    assert_eq!(first.added, 3);
    assert_eq!(first.pools[0].average_interval, Some(2.0));
    let second = merge(&path, &account("100"), &[pull("3", 5), pull("4", 4)]).unwrap();
    assert_eq!(second.total, 4);
    assert_eq!(second.added, 1);
    assert_eq!(second.pools[0].since_high_rarity, 1);
    merge(&path, &account("200"), &[pull("1", 3)]).unwrap();
    assert_eq!(summary(&path, Game::Genshin, Some("100")).unwrap().total, 4);
    assert_eq!(summary(&path, Game::Genshin, Some("200")).unwrap().total, 1);
    assert_eq!(merge(&path, &account("100"), &[]).unwrap().total, 4);
}

#[test]
fn rejects_invalid_batches_without_partial_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite");
    merge(&path, &account("100"), &[pull("1", 3)]).unwrap();
    assert!(merge(&path, &account("100"), &[pull("2", 3), pull("3", 0)]).is_err());
    assert!(merge(&path, &account("100"), &[pull("2", 3), pull("2", 3)]).is_err());
    assert_eq!(summary(&path, Game::Genshin, Some("100")).unwrap().total, 1);
}

#[test]
fn corrupt_store_is_not_replaced() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite");
    std::fs::write(&path, "damaged archive").unwrap();
    assert!(summary(&path, Game::Genshin, None).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "damaged archive");
}

#[test]
fn conflicting_history_rolls_back_the_whole_sync() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite");
    merge(&path, &account("100"), &[pull("1", 3)]).unwrap();
    assert!(merge(&path, &account("100"), &[pull("2", 3), pull("1", 5)]).is_err());
    let saved = summary(&path, Game::Genshin, Some("100")).unwrap();
    assert_eq!(saved.total, 1);
    assert_eq!(saved.recent[0].rarity, 3);
}

#[test]
fn rejects_dates_and_keeps_game_archives_separate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite");
    let mut item = pull("1", 5);
    item.time = "2026-02-30 00:00:00".into();
    assert!(merge(&path, &account("100"), &[item]).is_err());
    merge(&path, &account("100"), &[pull("1", 5)]).unwrap();
    let mut endfield = account("100");
    endfield.game = Game::Endfield;
    let mut free = pull("1", 6);
    free.is_free = Some(true);
    free.is_new = Some(true);
    let result = merge(&path, &endfield, &[free]).unwrap();
    assert_eq!(result.pools[0].high_rarity, 1);
    assert_eq!(result.pools[0].free_pulls, 1);
    assert_eq!(result.pools[0].average_interval, None);
    assert_eq!(
        summary(&path, Game::Genshin, Some("100")).unwrap().recent[0].rarity,
        5
    );
}

#[test]
fn official_report_preserves_real_pulls_and_older_five_stars() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite3");
    let mut owner = account("100");
    owner.game = Game::StarRail;
    owner.region = "prod_gf_cn".into();
    let mut real = pull("real-three-star", 3);
    real.pool = "11".into();
    merge(&path, &owner, &[real]).unwrap();
    let report: crate::StarRailReport = serde_json::from_value(serde_json::json!({"pools":[{"id":"11","name":"Character event","total":150,"sinceHighRarity":30,"fiveStars":[{"id":"old-star","itemId":1,"name":"Character","pulls":60,"isUp":true}]}]})).unwrap();
    let first = save_official(&path, &owner, report.clone()).unwrap();
    assert_eq!(first.added, 1);
    assert_eq!(first.total, 1);
    assert_eq!(first.recent[0].rarity, 3);
    let mut next = report.clone();
    next.pools[0].five_stars[0].id = "new-star".into();
    next.pools[0].total = 180;
    let second = save_official(&path, &owner, next.clone()).unwrap();
    assert_eq!(second.added, 1);
    assert_eq!(
        second.official.as_ref().unwrap().pools[0].five_stars.len(),
        2
    );
    assert_eq!(save_official(&path, &owner, next).unwrap().added, 0);
    let before =
        serde_json::to_value(summary(&path, Game::StarRail, Some("100")).unwrap()).unwrap();
    let mut invalid = report;
    invalid.pools[0].five_stars[0].name.clear();
    assert!(save_official(&path, &owner, invalid).is_err());
    let after = serde_json::to_value(summary(&path, Game::StarRail, Some("100")).unwrap()).unwrap();
    assert_eq!(before, after);
    assert_eq!(after["official"]["pools"][0]["total"], 180);
    assert_eq!(after["total"], 1);
    assert_eq!(after["recent"][0]["id"], "real-three-star");
    assert!(
        summary(&path, Game::Genshin, None)
            .unwrap()
            .official
            .is_none()
    );
}
