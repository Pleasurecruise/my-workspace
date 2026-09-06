use super::*;

fn session(id: &str, token: &str) -> Session {
    Session::Mihoyo {
        account_id: id.into(),
        stoken: token.into(),
        mid: "mid".into(),
        ltoken: "ltoken".into(),
        device: "device".into(),
        record: None,
    }
}

#[test]
fn separate_games() {
    let mut accounts = Accounts::default();
    accounts.insert(session("1", "first")).unwrap();
    accounts.insert(session("2", "second")).unwrap();
    assert_eq!(accounts.bindings["genshin"], "1");
    accounts.select("starRail", Some("2")).unwrap();
    accounts.insert(session("1", "renewed")).unwrap();
    let encoded = serde_json::to_string(&accounts).unwrap();
    let mut restored: Accounts = serde_json::from_str(&encoded).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.sessions.len(), 2);
    assert_eq!(restored.bindings["genshin"], "1");
    assert_eq!(restored.bindings["starRail"], "2");
    assert!(
        matches!(&restored.sessions["1"], Session::Mihoyo { stoken, .. } if stoken == "renewed")
    );
    restored.remove("1").unwrap();
    assert!(!restored.bindings.contains_key("genshin"));
    assert_eq!(restored.bindings["starRail"], "2");
    assert!(restored.select("genshin", Some("1")).is_err());
    assert!(restored.select("arknights", Some("2")).is_err());
}

#[test]
fn legacy_format() {
    let encoded = serde_json::to_string(&session("1", "token")).unwrap();
    let Saved::Legacy(legacy) = serde_json::from_str(&encoded).unwrap() else {
        panic!("Expected legacy session");
    };
    let mut accounts = Accounts::default();
    accounts.insert(legacy).unwrap();
    assert_eq!(accounts.bindings.len(), 3);
    accounts.bindings.insert("genshin".into(), "missing".into());
    assert!(accounts.validate().is_err());
    assert!(
        serde_json::from_str::<Saved>(r#"{"sessions":{},"bindings":{},"unexpected":true}"#)
            .is_err()
    );
}
