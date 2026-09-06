use super::*;

pub(crate) fn record() -> RecordSession {
    RecordSession {
        session: Session::Mihoyo {
            account_id: "123".into(),
            stoken: "private-stoken".into(),
            mid: "private-mid".into(),
            ltoken: "test-ltoken".into(),
            device: "login-device".into(),
            record: None,
        },
        cookies: BTreeMap::from([
            ("account_id".into(), "123".into()),
            ("cookie_token".into(), "test-cookie".into()),
            ("ltuid".into(), "123".into()),
            ("ltoken".into(), "test-ltoken".into()),
        ]),
        device: "12345678-1234-4123-8123-123456789abc".into(),
        fingerprint: "test-fingerprint".into(),
        updated_at: transport::now(),
        accounts: Default::default(),
        challenges: Default::default(),
        verification_traces: Default::default(),
    }
}

#[test]
fn starrail_headers() {
    let record = record();
    let request = record.request(Game::StarRail, reqwest::Url::parse("https://api-takumi-record.mihoyo.com/game_record/app/hkrpg/api/note?server=prod_gf_cn&role_id=100").unwrap(), None).unwrap().build().unwrap();
    assert_eq!(
        request.headers()["Referer"],
        "https://webstatic.mihoyo.com/"
    );
    assert_eq!(request.headers()["x-rpc-app_version"], "2.95.1");
    assert_eq!(request.headers()["x-rpc-device_fp"], "test-fingerprint");
    assert_eq!(request.headers()["x-rpc-device_id"], record.device);
    let cookie = request.headers()["Cookie"].to_str().unwrap();
    assert!(cookie.contains("cookie_token=test-cookie"));
    assert!(!cookie.contains("stoken"));
    let parts: Vec<_> = request.headers()["DS"]
        .to_str()
        .unwrap()
        .split(',')
        .collect();
    let input = format!(
        "salt=xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs&t={}&r={}&b=&q=role_id=100&server=prod_gf_cn",
        parts[0], parts[1]
    );
    use md5::{Digest, Md5};
    assert_eq!(parts[2], format!("{:x}", Md5::digest(input.as_bytes())));
}

#[test]
fn bridge_cookie_scope() {
    let page = VerificationPage {
        record: record(),
        account: Account {
            game: Game::StarRail,
            uid: "100".into(),
            region: "prod_gf_cn".into(),
            name: "Player".into(),
            role_id: "100".into(),
        },
        url: "https://webstatic.mihoyo.com/app/community-game-records/?game_id=6",
    };
    let request: BridgeMessage =
        serde_json::from_str(r#"{"method":"getCookieInfo","callback":"cb"}"#).unwrap();
    let response = serde_json::to_string(&page.respond(&request).unwrap()).unwrap();
    assert!(response.contains("cookie_token"));
    assert!(!response.contains("private-stoken"));
    assert!(!response.contains("private-mid"));
}

#[test]
fn session_invalidation() {
    let mut record = record();
    assert!(record.matches(&record.session));
    let mut changed = record.session.clone();
    if let Session::Mihoyo { stoken, .. } = &mut changed {
        *stoken = "new-stoken".into();
    }
    assert!(!record.matches(&changed));
    record.updated_at -= 3 * 24 * 60 * 60;
    assert!(!record.matches(&record.session));
}

#[test]
fn bridge_signature() {
    let page = VerificationPage {
        record: record(),
        account: Account {
            game: Game::StarRail,
            uid: "100".into(),
            region: "prod_gf_cn".into(),
            name: "Player".into(),
            role_id: "100".into(),
        },
        url: "https://webstatic.mihoyo.com/app/community-game-records/?game_id=6",
    };
    for query in [
        serde_json::json!({"server":"prod_gf_cn","need_detail":true}),
        serde_json::json!(r#"{"server":"prod_gf_cn","need_detail":true}"#),
    ] {
        let message: BridgeMessage = serde_json::from_value(serde_json::json!({"method":"getDS2","callback":"cb","payload":{"query":query,"body":"{\"id\":1}"}})).unwrap();
        let response = page.respond(&message).unwrap().unwrap();
        let ds = response.data["DS"].as_str().unwrap();
        let parts: Vec<_> = ds.split(',').collect();
        let input = format!(
            "salt=xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs&t={}&r={}&b={{\"id\":1}}&q=need_detail=true&server=prod_gf_cn",
            parts[0], parts[1]
        );
        use md5::{Digest, Md5};
        assert_eq!(parts[2], format!("{:x}", Md5::digest(input.as_bytes())));
    }
}

#[test]
fn pull_authkey_uses_stoken_and_hutao_lk2_profile() {
    let body = serde_json::json!({"auth_appid":"webview_gacha","game_biz":"hkrpg_cn","game_uid":100,"region":"prod_gf_cn"}).to_string();
    let request = record()
        .authkey_request(body.clone())
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(
        request.url().as_str(),
        "https://api-takumi.mihoyo.com/binding/api/genAuthKey"
    );
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.headers()["Cookie"],
        "stuid=123; stoken=private-stoken; mid=private-mid"
    );
    assert_eq!(request.headers()["x-rpc-app_version"], "2.95.1");
    assert_eq!(request.headers()["x-rpc-client_type"], "5");
    assert_eq!(request.headers()["x-rpc-device_fp"], "test-fingerprint");
    assert_eq!(request.headers()["Referer"], "https://app.mihoyo.com");
    assert_eq!(request.body().unwrap().as_bytes().unwrap(), body.as_bytes());
    let parts: Vec<_> = request.headers()["DS"]
        .to_str()
        .unwrap()
        .split(',')
        .collect();
    let input = format!(
        "salt=sidQFEglajEz7FA0Aj7HQPV88zpf17SO&t={}&r={}",
        parts[0], parts[1]
    );
    use md5::{Digest, Md5};
    assert_eq!(parts[2], format!("{:x}", Md5::digest(input.as_bytes())));
}
