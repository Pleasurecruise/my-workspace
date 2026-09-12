use super::*;
use std::io::{Read, Write};

fn server(
    responses: Vec<(u16, serde_json::Value)>,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/resets", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let length = stream.read(&mut buffer).unwrap();
                assert!(length > 0);
                request.extend_from_slice(&buffer[..length]);
            }
            requests.push(String::from_utf8(request).unwrap());
            let body = body.to_string();
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nRetry-After: 60\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    (endpoint, thread)
}

fn reset(id: &str, date: &str) -> serde_json::Value {
    serde_json::json!({"id": id, "announced_at": date, "reset_type":"regular", "text":"Limits reset.", "source":{"type":"x_post","author":"thsottiaux","url":"https://x.com/thsottiaux/status/1"}})
}

fn page(data: Vec<serde_json::Value>, cursor: Option<&str>) -> serde_json::Value {
    serde_json::json!({"data":data,"pagination":{"has_more":cursor.is_some(),"next_cursor":cursor}})
}

#[test]
fn local_day_bounds_cover_dst_changes() {
    let zone = TimeZone::get("Europe/London").unwrap();
    for (date, start, end) in [
        ("2026-03-29", "2026-03-29T00:00:00Z", "2026-03-29T23:00:00Z"),
        ("2026-10-25", "2026-10-24T23:00:00Z", "2026-10-26T00:00:00Z"),
    ] {
        let actual = bounds(date, &zone).unwrap();
        assert_eq!(actual, (start.parse().unwrap(), end.parse().unwrap()));
    }
}

#[tokio::test]
async fn paginates_and_projects_local_announcements() {
    let mut banked = reset("credit", "2026-09-11T23:10:00Z");
    banked["reset_type"] = "banked".into();
    banked["source"] = serde_json::json!({"type":"observed"});
    let (endpoint, server) = server(vec![
        (
            200,
            page(
                vec![reset("first", "2026-09-11T23:00:00Z")],
                Some("next_page"),
            ),
        ),
        (
            200,
            page(
                vec![banked, reset("tomorrow", "2026-09-12T23:00:00Z")],
                None,
            ),
        ),
    ]);
    let items = read_from(
        "2026-09-12",
        TimeZone::get("Europe/London").unwrap(),
        &endpoint,
    )
    .await
    .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "codex:first");
    assert_eq!(items[0].text, "00:00 Codex usage reset");
    assert_eq!(items[0].details.as_ref().unwrap().start_date, "2026-09-12");
    assert!(
        items[0]
            .description
            .as_ref()
            .unwrap()
            .contains("https://x.com/thsottiaux/status/1")
    );
    assert_eq!(items[1].text, "00:10 Codex reset credit");
    assert!(!items[1].rollover && !items[1].completed);
    assert!(requests[1].contains("cursor=next_page"));
    assert!(requests[0].contains("from=2026-09-11T23%3A00%3A00Z"));
}

#[tokio::test]
async fn rejects_partial_or_malformed_results() {
    for responses in [
        vec![(503, serde_json::json!({"secret":"not displayed"}))],
        vec![(200, page(vec![reset("bad", "invalid")], None))],
        vec![(
            200,
            page(vec![reset("wrong-date", "2026-09-10T12:00:00Z")], None),
        )],
        vec![(
            200,
            page(
                vec![
                    reset("same", "2026-09-12T12:00:00Z"),
                    reset("same", "2026-09-12T13:00:00Z"),
                ],
                None,
            ),
        )],
        vec![
            (200, page(vec![], Some("repeated"))),
            (200, page(vec![], Some("repeated"))),
        ],
        vec![(
            200,
            serde_json::json!({"data":[],"pagination":{"has_more":true,"next_cursor":null}}),
        )],
    ] {
        let (endpoint, server) = server(responses);
        let error = read_from("2026-09-12", TimeZone::UTC, &endpoint)
            .await
            .unwrap_err();
        server.join().unwrap();
        assert!(!error.to_string().contains("not displayed"));
    }
}

#[tokio::test]
async fn rate_limit_keeps_retry_guidance_without_response_body() {
    let (endpoint, server) = server(vec![(429, serde_json::json!({"detail":"private body"}))]);
    let error = read_from("2026-09-12", TimeZone::UTC, &endpoint)
        .await
        .unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("retry after 60 seconds"));
    assert!(!error.to_string().contains("private body"));
}
