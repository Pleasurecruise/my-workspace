use super::*;

#[tokio::test]
#[ignore = "Reads the official Star Rail activity report with the locally connected account; no daily requests"]
async fn live_report() {
    let session = transport::game(crate::Game::StarRail).unwrap();
    let record = RecordSession::create(&session).await.unwrap();
    let (account, report) =
        tokio::time::timeout(std::time::Duration::from_secs(300), read(&record))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(report.pools.len(), 6);
    let stars: usize = report.pools.iter().map(|pool| pool.five_stars.len()).sum();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.sqlite3");
    let summary = crate::archive::save_official(&path, &account, report).unwrap();
    assert!(summary.official.is_some());
    assert_eq!(
        summary.total, 0,
        "Official statistics must not fabricate individual pulls"
    );
    println!(
        "Star Rail official report: {} pools, {stars} five-star records, archived successfully",
        summary.official.unwrap().pools.len()
    );
}
