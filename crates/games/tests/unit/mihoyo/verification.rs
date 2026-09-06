use super::super::record::tests::record;
use super::*;

#[test]
fn hutao_challenge_requests_preserve_game_scope_and_signed_body() {
    for (game, id, path) in [
        (
            Game::Genshin,
            "2",
            "https://api-takumi-record.mihoyo.com/game_record/app/genshin/api/dailyNote",
        ),
        (
            Game::StarRail,
            "6",
            "https://api-takumi-record.mihoyo.com/game_record/app/hkrpg/api/note",
        ),
    ] {
        let record = record();
        let proof = CaptchaSolution {
            geetest_challenge: "challenge".into(),
            geetest_validate: "validate".into(),
            geetest_seccode: "validate|jordan".into(),
        };
        let body = proof.body(game).unwrap();
        for body in [None, Some(body.as_str())] {
            let request = card_request(&record, game, body, Some("test-trace"))
                .unwrap()
                .build()
                .unwrap();
            assert_eq!(request.headers()["x-rpc-challenge_game"], id);
            assert_eq!(request.headers()["x-rpc-challenge_trace"], "test-trace");
            assert_eq!(request.headers()["x-rpc-challenge_path"], path);
            assert_eq!(request.headers()["x-rpc-app_version"], "2.95.1");
            assert_eq!(request.headers()["x-rpc-device_fp"], "test-fingerprint");
            if game == Game::StarRail {
                assert_eq!(request.headers()["x-rpc-tool_verison"], "v4.5.0");
                assert_eq!(request.headers()["x-rpc-page"], "v4.5.0_#/rpg");
            }
            assert!(
                !request.headers()["Cookie"]
                    .to_str()
                    .unwrap()
                    .contains("stoken")
            );
            let parts: Vec<_> = request.headers()["DS"]
                .to_str()
                .unwrap()
                .split(',')
                .collect();
            let query = if body.is_some() {
                ""
            } else if game == Game::StarRail {
                "app_key=hkrpg_game_record&is_high=true"
            } else {
                "is_high=true"
            };
            let input = format!(
                "salt=xV8v4Qu54lUKrEYFZkJhB8cuOh9Asafs&t={}&r={}&b={}&q={query}",
                parts[0],
                parts[1],
                body.unwrap_or("")
            );
            use md5::{Digest, Md5};
            assert_eq!(parts[2], format!("{:x}", Md5::digest(input.as_bytes())));
            if let Some(body) = body {
                assert_eq!(request.method(), reqwest::Method::POST);
                assert_eq!(request.body().unwrap().as_bytes().unwrap(), body.as_bytes());
                assert_eq!(
                    request.url().as_str(),
                    if game == Game::StarRail {
                        RAIL_VERIFY
                    } else {
                        VERIFY
                    }
                );
                let payload: serde_json::Value = serde_json::from_str(body).unwrap();
                if game == Game::StarRail {
                    assert_eq!(payload["app_key"], "hkrpg_game_record");
                } else {
                    assert!(payload.get("app_key").is_none());
                }
            } else {
                assert_eq!(
                    request.url().as_str(),
                    if game == Game::StarRail {
                        RAIL_REGISTER
                    } else {
                        REGISTER
                    }
                );
            }
        }
    }
}

#[tokio::test]
async fn verified_challenge_is_one_use_and_only_for_its_game() {
    for game in [Game::Genshin, Game::StarRail] {
        let other_game = if game == Game::Genshin {
            Game::StarRail
        } else {
            Game::Genshin
        };
        let directory = tempfile::tempdir().unwrap();
        let runtime = Runtime::new(directory.path().join("games.sqlite3"));
        let record = record();
        runtime.verification.lock().await.insert(
            "id".into(),
            Pending {
                trace: None,
                game,
                record: record.clone(),
                expires_at: transport::now() + 600,
            },
        );
        runtime
            .finish_verification("id", "verified-token".into())
            .await
            .unwrap();
        assert!(
            runtime
                .finish_verification("id", "duplicate".into())
                .await
                .is_err()
        );
        let account = crate::Account {
            game,
            uid: "100".into(),
            region: "cn_gf01".into(),
            name: "Player".into(),
            role_id: "100".into(),
        };
        let other = record
            .note_request(other_game, &account)
            .await
            .unwrap()
            .build()
            .unwrap();
        assert!(!other.headers().contains_key("x-rpc-challenge"));
        let request = record
            .note_request(game, &account)
            .await
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(request.headers()["x-rpc-challenge"], "verified-token");
        assert_eq!(
            request.headers()["x-rpc-tool_verison"],
            if game == Game::Genshin {
                "v5.0.1-ys"
            } else {
                "v4.5.0"
            }
        );
        let next = record
            .note_request(game, &account)
            .await
            .unwrap()
            .build()
            .unwrap();
        assert!(!next.headers().contains_key("x-rpc-challenge"));
    }
}

#[tokio::test]
async fn cancelled_and_expired_verification_cannot_install_late_results() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new(directory.path().join("games.sqlite3"));
    let record = record();
    for expired in [false, true] {
        runtime.verification.lock().await.insert(
            "id".into(),
            Pending {
                trace: None,
                game: Game::StarRail,
                record: record.clone(),
                expires_at: if expired { 1 } else { transport::now() + 600 },
            },
        );
        if !expired {
            runtime.cancel_verification("id").await;
        }
        assert!(
            runtime
                .finish_verification("id", "late-token".into())
                .await
                .is_err()
        );
        assert!(record.challenges.lock().await.is_empty());
    }
}

#[test]
fn malformed_proof_is_rejected_before_any_request() {
    for value in ["", "a\r\nb", "a b"] {
        let proof = CaptchaSolution {
            geetest_challenge: "challenge".into(),
            geetest_validate: value.into(),
            geetest_seccode: format!("{value}|jordan"),
        };
        assert!(proof.validate().is_err());
    }
    let proof = CaptchaSolution {
        geetest_challenge: "challenge".into(),
        geetest_validate: "validate".into(),
        geetest_seccode: "wrong".into(),
    };
    assert!(proof.validate().is_err());
}

#[tokio::test]
async fn trace_is_scoped_to_the_restricted_game_and_cleared_by_a_new_response() {
    let record = record();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-trace-id", "rail-trace".parse().unwrap());
    record
        .remember_verification_trace(Game::StarRail, 10041, &headers)
        .await;
    let traces = record.verification_traces.lock().await;
    assert_eq!(traces.get(Game::StarRail.key()).unwrap(), "rail-trace");
    assert!(!traces.contains_key(Game::Genshin.key()));
    drop(traces);
    record
        .remember_verification_trace(Game::StarRail, 0, &headers)
        .await;
    assert!(record.verification_traces.lock().await.is_empty());
    record
        .remember_verification_trace(Game::StarRail, 1034, &headers)
        .await;
    record
        .remember_verification_trace(Game::StarRail, 1034, &Default::default())
        .await;
    assert!(record.verification_traces.lock().await.is_empty());
}

#[test]
fn rejection_messages_explain_known_failures_without_exposing_codes_or_response_data() {
    for (code, reason) in [
        (-201, "parameters"),
        (-3202, "captcha result"),
        (-3209, "expired"),
        (-110, "Too many requests"),
        (10001, "login expired"),
        (-205, "verification key"),
    ] {
        let response: Envelope = serde_json::from_value(serde_json::json!({"retcode":code,"message":"secret-cookie","data":{"token":"private-token"}})).unwrap();
        let error = response.decode::<Verified>().err().unwrap();
        assert!(error.contains(reason));
        assert!(!error.contains(&code.to_string()));
        assert!(!error.contains("secret-cookie"));
        assert!(!error.contains("private-token"));
    }
}

#[tokio::test]
async fn diagnostic_file_is_bounded_and_contains_only_safe_failure_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("diagnostics/mihoyo-verification.json");
    for retcode in [0, -999] {
        Diagnostic {
            game: Game::StarRail,
            stage: "submit",
            retcode,
            has_trace: true,
            timestamp: 123,
        }
        .save(&path)
        .await
        .unwrap();
    }
    let value: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(path).await.unwrap()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"game":"starRail","stage":"submit","retcode":-999,"has_trace":true,"timestamp":123})
    );
}

#[tokio::test]
async fn no_captcha_response_does_not_authorize_daily_access() {
    let response = Envelope {
        retcode: 30001,
        data: None,
    };
    let error = response.verification::<Verified>().err().unwrap();
    assert!(error.contains("did not authorize game-record access"));
    assert!(!error.contains("30001"));
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new(directory.path().join("games.sqlite3"));
    let record = record();
    let key = (
        Game::StarRail,
        "123".to_owned(),
        "private-stoken".to_owned(),
    );
    let _ = runtime
        .notes
        .read(key.clone(), false, async {
            Err(crate::NotesError::VerificationRequired(10041))
        })
        .await;
    runtime.verification.lock().await.insert(
        "id".into(),
        Pending {
            game: Game::StarRail,
            record: record.clone(),
            expires_at: transport::now() + 600,
            trace: None,
        },
    );
    assert!(
        runtime
            .finish_verification("id", String::new())
            .await
            .is_err()
    );
    assert!(record.challenges.lock().await.is_empty());
    let cached = runtime
        .notes
        .read(key, false, async {
            panic!("Missing authorization must not fetch notes")
        })
        .await;
    assert!(matches!(
        cached,
        Err(crate::NotesError::VerificationRequired(10041))
    ));
}

#[tokio::test]
async fn verification_replaces_the_cached_prompt_without_fetching_daily_notes() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new(directory.path().join("games.sqlite3"));
    let record = record();
    let key = (
        Game::StarRail,
        "123".to_owned(),
        "private-stoken".to_owned(),
    );
    let _ = runtime
        .notes
        .read(key.clone(), false, async {
            Err(crate::NotesError::VerificationRequired(10041))
        })
        .await;
    runtime.mark_verified(&record, Game::Genshin).await;
    let cached = runtime
        .notes
        .read(key.clone(), false, async { panic!("Must use cache") })
        .await;
    assert!(matches!(
        cached,
        Err(crate::NotesError::VerificationRequired(10041))
    ));
    runtime.mark_verified(&record, Game::StarRail).await;
    let cached = runtime
        .notes
        .read(key.clone(), false, async {
            panic!("Verification must not fetch notes")
        })
        .await;
    assert!(matches!(cached, Err(crate::NotesError::RefreshRequired)));
    let refreshed = runtime
        .notes
        .read(key, true, async {
            Err(crate::NotesError::VerificationRequired(1034))
        })
        .await;
    assert!(matches!(
        refreshed,
        Err(crate::NotesError::VerificationRequired(1034))
    ));
}

#[test]
fn registration_preserves_provider_geetest_flags() {
    for (flags, expected) in [
        (
            serde_json::json!({"new_captcha":false,"success":0}),
            (Some(false), Some(0)),
        ),
        (
            serde_json::json!({"new_captcha":true,"success":1}),
            (Some(true), Some(1)),
        ),
        (serde_json::json!({}), (None, None)),
    ] {
        let mut payload = serde_json::json!({"gt":"gt","challenge":"challenge"});
        payload
            .as_object_mut()
            .unwrap()
            .extend(flags.as_object().unwrap().clone());
        let registration: Registration = serde_json::from_value(payload).unwrap();
        assert_eq!((registration.new_captcha, registration.success), expected);
    }
}
