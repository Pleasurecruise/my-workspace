use super::{CheckRequest, LoginRequest};

#[test]
fn serializes_the_minimal_check_request() {
    let request = serde_json::to_value(CheckRequest { username: "admin" }).unwrap();

    assert_eq!(request, serde_json::json!({ "username": "admin" }));
}

#[test]
fn serializes_the_minimal_login_request() {
    let request = serde_json::to_value(LoginRequest {
        username: "admin",
        password: "encrypted".to_owned(),
        keepalive: true,
        otp: true,
        is_simple: true,
    })
    .unwrap();

    assert_eq!(
        request,
        serde_json::json!({
            "username": "admin",
            "password": "encrypted",
            "keepalive": true,
            "otp": true,
            "is_simple": true,
        })
    );
}
