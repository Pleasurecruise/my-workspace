use super::*;
use std::collections::HashMap;

#[test]
fn validates_callback_requests() {
    let url =
        callback_url(b"GET /callback?code=ready%2Bvalue&state=expected HTTP/1.1\r\n").unwrap();
    assert_eq!(
        parse_callback(&url, "/callback", "expected").unwrap(),
        "ready+value"
    );
    assert!(parse_callback(&url, "/callback", "different").is_err());
    assert!(parse_callback(&url, "/login", "expected").is_err());
    for query in [
        "error=access_denied&state=expected",
        "code=ready",
        "code=&state=expected",
        "code=ready&state=wrong&state=expected",
        "code=one&code=two&state=expected",
    ] {
        let url = reqwest::Url::parse(&format!("http://127.0.0.1/callback?{query}")).unwrap();
        assert!(parse_callback(&url, "/callback", "expected").is_err());
    }
    for line in [
        b"POST /callback HTTP/1.1\r\n".as_slice(),
        b"GET //other.example/callback HTTP/1.1\r\n",
        b"GET http://other.example/callback HTTP/1.1\r\n",
        b"GET /callback NOT-HTTP\r\n",
    ] {
        assert!(callback_url(line).is_err());
    }
}

async fn token_server(status: &str, body: &str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/token", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).await.unwrap();
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(body).unwrap()
    });
    (endpoint, task)
}

#[tokio::test]
async fn exchanges_pkce_code_through_loopback_callback() {
    let (endpoint, request) = token_server("200 OK", r#"{"access_token":"access","refresh_token":"refresh","token_type":"Bearer","expires_in":3600}"#).await;
    let authorization = authorize(
        "client",
        "https://example.com/authorize",
        &endpoint,
        "http://127.0.0.1:0/callback",
        "read write",
    )
    .await
    .unwrap();
    let url = reqwest::Url::parse(&authorization.url).unwrap();
    let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query["client_id"], "client");
    assert_eq!(query["redirect_uri"], "http://127.0.0.1:0/callback");
    assert_eq!(query["scope"], "read write");
    assert_eq!(query["code_challenge_method"], "S256");
    assert!(!query.contains_key("code_verifier"));
    let verifier = authorization.verifier.secret().clone();
    let callback = format!(
        "http://{}/callback?code=code&state={}",
        authorization.listener.local_addr().unwrap(),
        query["state"]
    );
    let task = tokio::spawn(authenticate(authorization));
    let browser = reqwest::Client::builder().no_proxy().build().unwrap();
    let unrelated = reqwest::Url::parse(&callback)
        .unwrap()
        .join("/favicon.ico")
        .unwrap();
    assert_eq!(
        browser.get(unrelated).send().await.unwrap().status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    let response = browser.get(callback).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let token = task.await.unwrap().unwrap();
    assert_eq!(token.access_token, "access");
    assert_eq!(token.refresh_token.as_deref(), Some("refresh"));
    assert_eq!(token.expires_in, 3600);
    let body = request.await.unwrap();
    let body = reqwest::Url::parse(&format!("http://localhost/?{body}")).unwrap();
    let fields: HashMap<_, _> = body.query_pairs().collect();
    assert_eq!(fields["grant_type"], "authorization_code");
    assert_eq!(fields["code"], "code");
    assert_eq!(fields["code_verifier"], verifier);
    assert_eq!(fields["client_id"], "client");
    assert_eq!(fields["redirect_uri"], "http://127.0.0.1:0/callback");
}

#[tokio::test]
async fn cancellation_releases_callback_port() {
    let authorization = authorize(
        "client",
        "https://example.com/authorize",
        "https://example.com/token",
        "http://127.0.0.1:0/callback",
        "read",
    )
    .await
    .unwrap();
    let address = authorization.listener.local_addr().unwrap();
    let task = tokio::spawn(authenticate(authorization));
    tokio::task::yield_now().await;
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    assert!(TcpListener::bind(address).await.is_ok());
}

#[tokio::test]
async fn refreshes_with_optional_rotation() {
    for rotated in [false, true] {
        let body = if rotated {
            r#"{"access_token":"new","refresh_token":"rotated","token_type":"Bearer","expires_in":7200}"#
        } else {
            r#"{"access_token":"new","token_type":"Bearer","expires_in":7200}"#
        };
        let (endpoint, request) = token_server("200 OK", body).await;
        let token = refresh("client", &endpoint, "old").await.unwrap();
        assert_eq!(token.access_token, "new");
        assert_eq!(token.expires_in, 7200);
        assert_eq!(token.refresh_token.as_deref(), rotated.then_some("rotated"));
        let body = request.await.unwrap();
        let url = reqwest::Url::parse(&format!("http://localhost/?{body}")).unwrap();
        let fields: HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(fields["grant_type"], "refresh_token");
        assert_eq!(fields["refresh_token"], "old");
        assert_eq!(fields["client_id"], "client");
    }
}

#[tokio::test]
async fn redacts_failed_token_responses() {
    for (status, body) in [
        (
            "400 Bad Request",
            r#"{"error":"invalid_grant","error_description":"synthetic-secret"}"#,
        ),
        ("200 OK", "synthetic-secret"),
        (
            "200 OK",
            r#"{"access_token":"synthetic-secret","token_type":"Bearer"}"#,
        ),
    ] {
        let (endpoint, request) = token_server(status, body).await;
        let error = match refresh("client", &endpoint, "old").await {
            Ok(_) => panic!("invalid token response accepted"),
            Err(error) => error,
        };
        assert!(!format!("{error:?} {error}").contains("synthetic-secret"));
        request.await.unwrap();
    }
}
