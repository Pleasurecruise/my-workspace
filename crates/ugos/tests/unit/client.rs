use super::parse_client_version;

#[test]
fn reads_the_current_web_client_version() {
    let desktop = r#"const clientNumberVersion=window.clientNumberVersion=78376"#;

    assert_eq!(parse_client_version(desktop).unwrap(), "78376");
}

#[test]
fn rejects_a_desktop_page_without_a_client_version() {
    let error = parse_client_version("<html></html>").unwrap_err();

    assert_eq!(
        error.to_string(),
        "UGOS response from desktop could not be decoded: clientNumberVersion is missing"
    );
}
