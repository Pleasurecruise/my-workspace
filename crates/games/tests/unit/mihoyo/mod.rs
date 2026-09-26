use super::*;

#[tokio::test]
#[ignore = "Reads local miHoYo pull history without printing account data"]
async fn live_pulls() {
    let session = transport::game(Game::Genshin).unwrap();
    let record = record::RecordSession::create(&session).await.unwrap();
    let (account, pulls) = pulls(&record, Game::Genshin).await.unwrap();
    for (field, count) in [
        ("id", pulls.iter().filter(|pull| pull.id.is_empty()).count()),
        (
            "pool",
            pulls.iter().filter(|pull| pull.pool.is_empty()).count(),
        ),
        (
            "item_id",
            pulls.iter().filter(|pull| pull.item_id.is_none()).count(),
        ),
        (
            "name",
            pulls.iter().filter(|pull| pull.name.is_empty()).count(),
        ),
    ] {
        println!("Missing {field}: {count}");
    }
    let directory = tempfile::tempdir().unwrap();
    crate::archive::merge(&directory.path().join("games.sqlite3"), &account, &pulls).unwrap();
}

#[tokio::test]
#[ignore = "Reads Star Rail notes using the locally connected miHoYo account"]
async fn live_record() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = crate::Runtime::new(directory.path().join("games.sqlite3"));
    match runtime.notes(Game::StarRail, false).await {
        Ok(_) => println!("Star Rail notes: ready"),
        Err(crate::NotesError::VerificationRequired(code)) => {
            println!("Star Rail notes: verification required (code {code})")
        }
        Err(crate::NotesError::RefreshRequired) => {
            println!("Star Rail notes: waiting for manual refresh")
        }
        Err(crate::NotesError::Failed(message)) => panic!("Star Rail notes failed: {message}"),
    }
}

#[test]
fn official_role() {
    let roles: Roles = serde_json::from_value(serde_json::json!({"list": [
        {"game_uid":"channel", "region":"prod_qd_cn", "nickname":"Channel", "is_chosen":true},
        {"game_uid":"official", "region":"prod_gf_cn", "nickname":"Official", "is_chosen":false}
    ]}))
    .unwrap();
    let account = roles.select(Game::StarRail, "prod_gf_cn").unwrap();
    assert_eq!(account.uid, "official");
    let roles = Roles { list: Vec::new() };
    assert!(roles.select(Game::StarRail, "prod_gf_cn").is_err());
}

#[test]
fn note_verification() {
    for code in [1034, 5003, 10035, 10041, 10053] {
        let response: Envelope = serde_json::from_value(
            serde_json::json!({"retcode":code,"message":"private-token","data":{}}),
        )
        .unwrap();
        let error = response.note::<RailNotes>().err().unwrap();
        let response: crate::NotesResponse = Err(error).into();
        let encoded = serde_json::to_value(response).unwrap();
        assert_eq!(encoded["status"], "verificationRequired");
        assert_eq!(encoded["code"], code);
        let message = encoded["message"].as_str().unwrap();
        assert!(message.contains("security verification"));
        assert!(message.contains("refresh"));
        assert!(!message.contains(&code.to_string()));
        assert!(!encoded.to_string().contains("private-token"));
        let response = Envelope {
            retcode: code,
            data: None,
        };
        assert_eq!(response.decode::<RailNotes>().err().unwrap(), message);
    }
    let response: Envelope = serde_json::from_str(r#"{"retcode":-100,"data":{}}"#).unwrap();
    assert!(matches!(
        response.note::<RailNotes>(),
        Err(crate::NotesError::Failed(_))
    ));
}

#[tokio::test]
#[ignore = "Creates an anonymous QR code against the live miHoYo service"]
async fn qr_waiting() {
    let (pending, _) = begin().await.unwrap();
    assert_eq!(pending.device.len(), 53);
    assert!(matches!(poll(&pending).await.unwrap(), Poll::Waiting));
}

#[test]
fn ltoken_signature() {
    let request = passport(
        &transport::client().unwrap(),
        TokenKind::LToken,
        "test-device",
        "test-stoken",
        &Identity {
            aid: "123".into(),
            mid: "test-mid".into(),
        },
    )
    .build()
    .unwrap();
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(request.url().path(), "/account/auth/api/getLTokenBySToken");
    assert!(request.body().is_none());
    assert_eq!(request.headers()["x-rpc-app_id"], "bll8iq97cem8");
    assert_eq!(request.headers()["x-rpc-client_type"], "2");
    assert_eq!(request.headers()["x-rpc-device_id"], "test-device");
    assert_eq!(
        request.headers()["Cookie"],
        "stoken=test-stoken; stuid=123; mid=test-mid"
    );
    let parts: Vec<_> = request.headers()["DS"]
        .to_str()
        .unwrap()
        .split(',')
        .collect();
    assert_eq!(parts.len(), 3);
    let input = format!(
        "salt=JwYDpKvLj6MrMqqYU6jTKF17KNO2PXoS&t={}&r={}&b={{}}&q=",
        parts[0], parts[1]
    );
    assert_eq!(parts[2], format!("{:x}", Md5::digest(input.as_bytes())));
}

#[test]
fn qr_states() {
    for (status, scanned) in [("Created", false), ("Scanned", true)] {
        let response: Envelope = serde_json::from_value(serde_json::json!({
            "retcode": 0,
            "data": { "status": status, "tokens": [], "user_info": null }
        }))
        .unwrap();
        let state: QrStatus = response.decode().unwrap();
        assert_eq!(matches!(state, QrStatus::Scanned), scanned);
    }
}

#[test]
fn qr_tokens() {
    let response: Envelope = serde_json::from_str(
        r#"{"retcode":0,"data":{"status":"Confirmed","tokens":[{"token_type":2,"token":"test-ltoken"},{"token_type":1,"token":"test-stoken"}],"user_info":{"aid":"123","mid":"test-mid"},"need_realperson":true}}"#,
    ).unwrap();
    let state: QrStatus = response.decode().unwrap();
    let QrStatus::Confirmed {
        tokens,
        user_info,
        need_realperson,
    } = state
    else {
        panic!("Expected confirmed login");
    };
    assert!(need_realperson);
    assert_eq!(
        tokens
            .iter()
            .find(|token| token.token_type == 1)
            .unwrap()
            .token,
        "test-stoken"
    );
    assert_eq!(user_info.aid, "123");
}

#[test]
fn invalid_qr() {
    for data in [
        serde_json::json!({"status":"Confirmed", "tokens":[], "user_info":null, "need_realperson":false}),
        serde_json::json!({"status":"Unexpected"}),
    ] {
        let response: Envelope =
            serde_json::from_value(serde_json::json!({"retcode":0,"data":data})).unwrap();
        assert!(response.decode::<QrStatus>().is_err());
    }
}

#[test]
fn qr_risk() {
    let response: Envelope = serde_json::from_str(
        r#"{"retcode":-3503,"message":"private-message","data":{"token":"private-token"}}"#,
    )
    .unwrap();
    let error = response.decode::<QrStatus>().err().unwrap();
    assert!(error.contains("device or network"));
    assert!(!error.contains("-3503"));
    assert!(error.contains("new QR code"));
    assert!(!error.contains("private"));
}

#[test]
fn provider_error() {
    let response: Envelope =
        serde_json::from_str(r#"{"retcode":10102,"message":"private","data":{}}"#).unwrap();
    assert!(
        response
            .decode::<GenshinNotes>()
            .err()
            .unwrap()
            .contains("Enable real-time notes")
    );
}

#[test]
fn notes_signature() {
    let session = Session::Mihoyo {
        account_id: "1".into(),
        stoken: "stoken".into(),
        mid: "mid".into(),
        ltoken: "ltoken".into(),
        device: "device".into(),
        record: None,
    };
    let request = authenticated(
        &session,
        reqwest::Url::parse("https://api-takumi-record.mihoyo.com/note?server=cn_gf01&role_id=100")
            .unwrap(),
        None,
    )
    .unwrap()
    .build()
    .unwrap();
    assert_eq!(request.url().query(), Some("role_id=100&server=cn_gf01"));
    let parts: Vec<_> = request.headers()["DS"]
        .to_str()
        .unwrap()
        .split(',')
        .collect();
    assert_eq!(parts.len(), 3);
    assert!((100000..200000).contains(&parts[1].parse::<u64>().unwrap()));
    let input = format!(
        "salt=xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs&t={}&r={}&b=&q=role_id=100&server=cn_gf01",
        parts[0], parts[1]
    );
    assert_eq!(parts[2], format!("{:x}", Md5::digest(input.as_bytes())));
}

#[test]
fn zzz_response() {
    let response: Envelope = serde_json::from_str(r#"{"retcode":0,"data":{"gacha_item_list":[{"id":"1700000000000000001","item_type":"ITEM_TYPE_WEAPON","item_id":12003,"item_name":"Item","rarity":"B","date":{"year":2026,"month":1,"day":1,"hour":12,"minute":0,"second":0}}],"has_more":true}}"#).unwrap();
    let page: ZzzPage = response.decode().unwrap();
    assert!(page.has_more);
    assert_eq!(page.gacha_item_list[0].date.year, 2026);
    let notes: ZzzNotes = serde_json::from_str(r#"{"energy":{"progress":{"max":240,"current":55},"restore":66538},"vitality":{"max":400,"current":400},"vhs_sale":{"sale_state":"SaleStateDoing"},"card_sign":"CardSignDone"}"#).unwrap();
    assert_eq!(notes.energy.progress.current, 55);
    assert_eq!(notes.vitality.current, 400);
}

#[test]
fn encodes_pull_authorization() {
    for key in ["a+b/c=", "a%2Bb%2Fc%3D"] {
        let authorization: AuthKey = serde_json::from_value(
            serde_json::json!({"authkey":key,"authkey_ver":2,"sign_type":7}),
        )
        .unwrap();
        let mut url = reqwest::Url::parse("https://public-operation-hkrpg.mihoyo.com/common/gacha_record/api/getGachaLog?gacha_type=11&end_id=0").unwrap();
        authorization.append_query(&mut url).unwrap();
        let query: BTreeMap<_, _> = url.query_pairs().collect();
        assert_eq!(query.get("auth_appid").unwrap(), "webview_gacha");
        assert_eq!(query.get("authkey").unwrap(), "a+b/c=");
        assert_eq!(query.get("authkey_ver").unwrap(), "2");
        assert_eq!(query.get("sign_type").unwrap(), "7");
        assert_eq!(query.get("gacha_type").unwrap(), "11");
        assert!(!url.as_str().contains("%252B"));
    }
    for key in ["", "%FF", "a%0Ab"] {
        let authorization = AuthKey {
            authkey: key.into(),
            authkey_ver: 1,
            sign_type: 2,
        };
        let mut url = reqwest::Url::parse("https://example.com").unwrap();
        assert!(authorization.append_query(&mut url).is_err());
    }
}

#[test]
fn reports_rejected_pull_key() {
    for code in [-100, -101] {
        let response = Envelope {
            retcode: code,
            data: None,
        };
        let error = response.gacha().err().unwrap();
        assert!(error.contains("Pull-history authorization"));
        assert!(error.contains("Sync history"));
        assert!(!error.contains("Scan again"));
        assert!(!error.contains(&code.to_string()));
    }
}

#[test]
fn accepts_missing_item_ids() {
    for item_id in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::json!("")),
        Some(serde_json::json!("11401")),
    ] {
        let mut record = serde_json::json!({
            "id": "record-1", "uid": "100000001", "gacha_type": "301",
            "name": "Item", "rank_type": "4", "time": "2026-09-07 12:00:00"
        });
        if let Some(value) = item_id {
            record["item_id"] = value;
        }
        let response: Envelope = serde_json::from_value(serde_json::json!({
            "retcode": 0, "data": { "list": [record] }
        }))
        .unwrap();
        assert_eq!(response.gacha().unwrap().list.len(), 1);
    }
}
