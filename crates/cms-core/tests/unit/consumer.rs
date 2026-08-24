use super::{Channel, ChannelView};

#[test]
fn exposes_only_the_current_channels() {
    assert_eq!(Channel::try_from("memos").unwrap(), Channel::Memos);
    assert_eq!(Channel::try_from("moment").unwrap(), Channel::Moment);
    assert_eq!(Channel::try_from("knowledge").unwrap(), Channel::Knowledge);
    assert!(Channel::try_from("archive").is_err());
}

#[test]
fn serializes_page_cursors_for_the_typescript_boundary() {
    let view = ChannelView::Memos {
        connected: true,
        memos: Vec::new(),
        tags: Vec::new(),
        next_cursor: Some("second-page".to_owned()),
    };
    let value = serde_json::to_value(view).expect("channel view should serialize");

    assert_eq!(value["nextCursor"], "second-page");
    assert!(value.get("next_cursor").is_none());
}
