use super::parse_client_version;

#[test]
fn reads_client_version() {
    let desktop = r#"const clientNumberVersion=window.clientNumberVersion=78376"#;

    assert_eq!(parse_client_version(desktop).unwrap(), "78376");
}

#[test]
fn rejects_missing_version() {
    let error = parse_client_version("<html></html>").unwrap_err();

    assert_eq!(
        error.to_string(),
        "UGOS response from desktop could not be decoded: clientNumberVersion is missing"
    );
}
