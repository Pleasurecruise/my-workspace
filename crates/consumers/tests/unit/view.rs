use super::{Channel, ChannelView};

#[test]
fn lists_current_channels() {
    assert_eq!(Channel::try_from("memos").unwrap(), Channel::Memos);
    assert_eq!(Channel::try_from("moment").unwrap(), Channel::Moment);
    assert_eq!(Channel::try_from("knowledge").unwrap(), Channel::Knowledge);
    assert!(Channel::try_from("archive").is_err());
}

#[test]
fn serializes_cursors() {
    let view = ChannelView::Memos {
        memos: Vec::new(),
        next_cursor: Some("second-page".to_owned()),
    };
    let value = serde_json::to_value(view).expect("channel view should serialize");

    assert_eq!(value["nextCursor"], "second-page");
    assert!(value.get("next_cursor").is_none());
}

#[test]
fn serializes_newspaper() {
    let view = ChannelView::Knowledge {
        knowledge: Vec::new(),
        newspaper: crate::api::knowledge::NewspaperIssues {
            developer: Some("developer-issue".to_owned()),
            personal: None,
        },
        next_cursor: None,
    };
    let value = serde_json::to_value(view).expect("knowledge view should serialize");

    assert_eq!(value["newspaper"]["developer"], "developer-issue");
    assert!(value["newspaper"]["personal"].is_null());
}

#[test]
fn serializes_gallery_contract() {
    let value = serde_json::to_value(ChannelView::Moment {
        photos: Vec::new(),
        total: 0,
    })
    .unwrap();
    assert_eq!(
        value,
        serde_json::json!({"channel": "moment", "photos": [], "total": 0})
    );
}
