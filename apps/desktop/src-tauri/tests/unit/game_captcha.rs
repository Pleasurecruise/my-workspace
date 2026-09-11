use super::*;

#[test]
fn captcha_page_contains_no_account_bridge_or_credentials() {
    let captcha = Captcha {
        id: "private-id".into(),
        gt: "gt</script>".into(),
        challenge: "challenge".into(),
        new_captcha: Some(false),
        success: Some(0),
    };
    let url = page(&captcha).unwrap();
    let html = percent_encoding::percent_decode_str(url.as_str().split_once(',').unwrap().1)
        .decode_utf8()
        .unwrap();
    assert!(html.contains("gt\\u003c/script\\u003e"));
    assert!(html.contains("https://static.geetest.com/static/js/gt.0.5.2.js"));
    assert!(html.contains("\"new_captcha\":false"));
    assert!(html.contains("\"success\":0"));
    assert!(!html.contains("private-id"));
    assert!(!html.contains("MiHoYoJSInterface"));
    assert!(!html.contains("cookie_token"));
}

#[test]
fn captcha_callback_accepts_only_bounded_typed_proofs() {
    let proof = r#"{"geetest_challenge":"challenge","geetest_validate":"validate","geetest_seccode":"validate|jordan"}"#;
    let mut url = reqwest::Url::parse("vesper-captcha://result").unwrap();
    url.query_pairs_mut().append_pair("data", proof);
    assert!(solution(&url).is_ok());
    let mut oversized = url.clone();
    oversized.set_query(None);
    oversized
        .query_pairs_mut()
        .append_pair("data", &"x".repeat(16385));
    assert!(solution(&oversized).is_err());
    url.set_host(Some("other")).unwrap();
    assert!(solution(&url).is_err());
}
