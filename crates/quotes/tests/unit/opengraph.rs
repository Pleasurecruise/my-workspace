use super::*;

#[test]
fn parses_metadata_with_entities_first_image_and_relative_urls() {
    let url = Url::parse("https://example.com/articles/post").unwrap();
    let item = parse(
        r#"<html><head>
        <title>Fallback</title><meta name="description" content="Fallback description">
        <meta property="og:title" content="Rust &amp; Markdown">
        <meta property="og:description" content="&lt;script&gt;not markup&lt;/script&gt;">
        <meta property="og:image" content="../cover.png?a=1&amp;b=2">
        <meta property="og:image" content="https://example.com/second.png">
        <meta property="og:url" content="javascript:alert(1)">
        </head></html>"#,
        &url,
    );
    assert_eq!(item.title, "Rust & Markdown");
    assert_eq!(item.description, "<script>not markup</script>");
    assert_eq!(
        item.image.as_deref(),
        Some("https://example.com/cover.png?a=1&b=2")
    );
    assert_eq!(item.url, url.as_str());
    assert_eq!(item.site_name, "example.com");
}

#[test]
fn falls_back_without_open_graph_and_discards_unsafe_images() {
    let url = Url::parse("https://example.com/post").unwrap();
    let item = parse(
        r#"<title> Page &amp; title </title><meta name="description" content="Summary"><meta property="og:image" content="data:image/svg+xml,unsafe">"#,
        &url,
    );
    assert_eq!(item.title, "Page & title");
    assert_eq!(item.description, "Summary");
    assert!(item.image.is_none());
    assert_eq!(parse("", &url).title, "example.com");
}

#[test]
fn rejects_non_http_credentials_and_private_addresses() {
    for url in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "https://user:pass@example.com",
        "http://127.0.0.1",
        "http://2130706433",
        "http://10.0.0.1",
        "http://169.254.169.254",
        "http://[::1]",
        "http://[::ffff:127.0.0.1]",
    ] {
        assert!(validate_url(url).is_err(), "{url}");
    }
    assert!(validate_url("https://example.com/post").is_ok());
    assert!(!public_ip("192.168.1.1".parse().unwrap()));
    assert!(public_ip("2606:4700:4700::1111".parse().unwrap()));
}
