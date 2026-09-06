use super::*;

#[test]
fn rejects_removed_split_game_widgets() {
    for kind in ["gameNotes", "gachaAnalysis"] {
        let encoded = serde_json::to_vec(&serde_json::json!({"widgets": [{"id": "game", "widget": {"kind": kind, "game": "starRail"}}]})).unwrap();
        assert!(decode(&encoded).is_err());
    }
    assert!(
        decode(br#"{"widgets":[{"id":"game","widget":{"kind":"game","game":"starRail"}}]}"#)
            .is_ok()
    );
}
