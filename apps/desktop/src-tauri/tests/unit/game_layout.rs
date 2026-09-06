use super::*;

#[test]
fn combined_cards() {
    let layout = decode(br#"{"widgets":[{"id":"notes","widget":{"kind":"gameNotes","game":"starRail"}},{"id":"github","widget":{"kind":"github"}},{"id":"pulls","widget":{"kind":"gachaAnalysis","game":"starRail"}},{"id":"genshin","widget":{"kind":"gachaAnalysis","game":"genshin"}}]}"#).unwrap();
    assert_eq!(layout.widgets.len(), 3);
    assert_eq!(layout.widgets[0].id, "notes");
    assert!(matches!(
        layout.widgets[0].widget,
        Widget::Game {
            game: games::Game::StarRail
        }
    ));
    assert!(matches!(layout.widgets[1].widget, Widget::Github));
    assert!(matches!(
        layout.widgets[2].widget,
        Widget::Game {
            game: games::Game::Genshin
        }
    ));
    let restored = decode(&serde_json::to_vec(&layout).unwrap()).unwrap();
    assert_eq!(restored.widgets.len(), 3);
}
