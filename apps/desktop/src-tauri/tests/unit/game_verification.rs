use super::*;

#[test]
fn allowed_origins() {
    for url in [
        "https://webstatic.mihoyo.com/app/community-game-records/?game_id=6",
        "https://act.mihoyo.com/app/mihoyo-zzz-game-record/m.html?game_id=8",
        "https://api.geetest.com/",
    ] {
        assert!(allowed(&reqwest::Url::parse(url).unwrap()));
    }
    for url in [
        "https://webstatic.mihoyo.com.evil.example/",
        "http://webstatic.mihoyo.com/",
        "https://example.com/",
        "javascript:alert(1)",
    ] {
        assert!(!allowed(&reqwest::Url::parse(url).unwrap()));
    }
}

#[test]
fn parent_domain_session_is_available_to_record_subdomains() {
    let expected: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::from([
        ("ltuid".into(), "123".into()),
        ("ltoken".into(), "test-token".into()),
    ]);
    let cookies: Vec<_> = expected
        .iter()
        .map(|(name, value)| {
            Cookie::build((name.clone(), value.clone()))
                .domain(".mihoyo.com")
                .path("/")
                .secure(true)
                .build()
        })
        .collect();
    // Reproduce the Wry filter that made the former readback close a valid window.
    let target =
        reqwest::Url::parse("https://webstatic.mihoyo.com/app/community-game-records/").unwrap();
    assert!(
        cookies
            .iter()
            .all(|cookie| cookie.domain() != target.domain())
    );
    assert!(session_installed(&expected, &cookies));
    assert!(!session_installed(&expected, &cookies[..1]));
    for (domain, path, value) in [
        ("evil.example", "/", "test-token"),
        ("mihoyo.com.evil.example", "/", "test-token"),
        (".mihoyo.com", "/other", "test-token"),
        (".mihoyo.com", "/", "stale-token"),
    ] {
        let mut incorrect = cookies.clone();
        let token = incorrect
            .iter_mut()
            .find(|cookie| cookie.name() == "ltoken")
            .unwrap();
        token.set_domain(domain);
        token.set_path(path);
        token.set_value(value);
        assert!(!session_installed(&expected, &incorrect));
    }
}
