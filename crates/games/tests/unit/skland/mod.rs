use super::*;

#[test]
fn scan_states() {
    for status in [100, 101, 102] {
        let response: Envelope =
            serde_json::from_value(serde_json::json!({"status":status})).unwrap();
        let state = response.scan().unwrap();
        assert!(matches!(
            (status, state),
            (100, ScanState::Waiting) | (101, ScanState::Scanned) | (102, ScanState::Expired)
        ));
    }
    let response: Envelope =
        serde_json::from_str(r#"{"status":0,"data":{"scanCode":"confirmed"}}"#).unwrap();
    assert!(
        matches!(response.scan().unwrap(), ScanState::Confirmed(scan) if scan.scan_code == "confirmed")
    );
    let response: Envelope = serde_json::from_str(r#"{"status":999,"msg":"private"}"#).unwrap();
    assert_eq!(
        response.scan().err().unwrap(),
        "Skland QR login failed (code 999)"
    );
}

#[tokio::test]
#[ignore = "Creates an anonymous Skland QR code"]
async fn qr_waiting() {
    let (pending, _) = begin().await.unwrap();
    assert!(matches!(poll(&pending).await.unwrap(), Poll::Waiting));
}

#[test]
fn decodes_pending_scan() {
    let response: Envelope =
        serde_json::from_str(r#"{"msg":"未扫码","status":100,"type":"A"}"#).unwrap();
    assert_eq!(response.status, Some(100));
    assert!(response.data.is_none());
}

#[test]
fn reports_authentication_error() {
    let response: Envelope =
        serde_json::from_str(r#"{"code":10002,"data":{},"message":"sensitive details"}"#).unwrap();
    let error = response.decode::<Page<ArkPull>>().err().unwrap();
    assert!(error.contains("login expired"));
    assert!(!error.contains("sensitive details"));
}

#[test]
fn decodes_arknights_history() {
    let page: Page<ArkPull> = serde_json::from_str(r#"{"list":[{"poolId":"pool","poolName":"Standard","charId":"char","charName":"Operator","rarity":5,"isNew":true,"gachaTs":"1788676800123","pos":9}],"hasMore":true}"#).unwrap();
    assert_eq!(page.list[0].gacha_ts, "1788676800123");
    assert_eq!(page.list[0].pos, 9);
    assert_eq!(page.list[0].rarity + 1, 6);
    assert!(page.has_more);
}

#[test]
fn decodes_endfield_history() {
    let page: Page<EfPull> = serde_json::from_str(r#"{"list":[{"kind":"draw","poolId":"pool","poolName":"Special","charId":"char","charName":"Operator","rarity":6,"isFree":true,"isNew":true,"gachaTs":"1788676800123","seqId":"20"},{"kind":"bonus","gachaTs":"1788676800123","seqId":"19"}],"hasMore":true}"#).unwrap();
    assert_eq!(page.list[0].is_free, Some(true));
    assert_eq!(page.list.last().unwrap().seq_id, "19");
    assert_eq!(
        page.list
            .iter()
            .filter(|item| item.kind.as_deref() == Some("draw"))
            .count(),
        1
    );
}
