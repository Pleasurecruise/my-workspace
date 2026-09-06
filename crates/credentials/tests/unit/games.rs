use super::*;

#[test]
fn legacy_session() {
    let session: Session = serde_json::from_str(r#"{"provider":"mihoyo","account_id":"1","stoken":"test-stoken","mid":"test-mid","ltoken":"test-ltoken","device":"test-device"}"#).unwrap();
    session.validate().unwrap();
    assert!(matches!(session, Session::Mihoyo { record: None, .. }));
}

#[test]
fn record_device() {
    let session: Session = serde_json::from_str(r#"{"provider":"mihoyo","account_id":"1","stoken":"test-stoken","mid":"test-mid","ltoken":"test-ltoken","device":"test-device","record":{"id":"record-device","fingerprint":"fingerprint","updated_at":100}}"#).unwrap();
    session.validate().unwrap();
    let encoded = serde_json::to_string(&session).unwrap();
    let mut session: Session = serde_json::from_str(&encoded).unwrap();
    let Session::Mihoyo {
        record: Some(record),
        ..
    } = &mut session
    else {
        panic!("Missing device");
    };
    assert_eq!(record.id, "record-device");
    assert_eq!(record.fingerprint, "fingerprint");
    record.fingerprint = "bad; value".into();
    assert!(session.validate().is_err());
}
