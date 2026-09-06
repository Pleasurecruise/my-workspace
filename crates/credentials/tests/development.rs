#![cfg(debug_assertions)]

use vesper_credentials::{Stored, TelegramCredentials, XCredentials};

#[test]
fn steam_environment_overrides_file_without_hiding_invalid_configuration() {
    use vesper_credentials::games::{self, Provider, Session};

    if let Ok(case) = std::env::var("VESPER_STEAM_TEST_CASE") {
        games::save(&Session::Steam {
            api_key: "file-key".into(),
            steam_id: "76561198000000000".into(),
        })
        .unwrap();
        let result = games::read(Provider::Steam);
        match case.as_str() {
            "missing" | "complete" => {
                let expected = if case == "missing" {
                    "file-key"
                } else {
                    "env-key"
                };
                assert!(
                    matches!(result.unwrap(), Stored::Ready(Session::Steam { api_key, .. }) if api_key == expected)
                );
            }
            _ => assert!(result.is_err()),
        }
        return;
    }

    for (case, api_key, steam_id) in [
        ("missing", None, None),
        ("complete", Some("env-key"), Some("76561198000000001")),
        ("key-only", Some("env-key"), None),
        ("id-only", None, Some("76561198000000001")),
        ("empty", Some(""), Some("76561198000000001")),
        ("invalid-id", Some("env-key"), Some("invalid")),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "steam_environment_overrides_file_without_hiding_invalid_configuration",
            ])
            .env("VESPER_STEAM_TEST_CASE", case)
            .env("HOME", directory.path())
            .env("USERPROFILE", directory.path())
            .env("LOCALAPPDATA", directory.path())
            .env("XDG_DATA_HOME", directory.path())
            .env_remove("STEAM_API_KEY")
            .env_remove("STEAM_ID");
        if let Some(api_key) = api_key {
            command.env("STEAM_API_KEY", api_key);
        }
        if let Some(steam_id) = steam_id {
            command.env("STEAM_ID", steam_id);
        }
        assert!(command.status().unwrap().success(), "case: {case}");
    }
}

#[test]
fn development_reads_and_writes_stay_in_local_storage() {
    if std::env::var_os("VESPER_CREDENTIAL_TEST_CHILD").is_none() {
        let directory = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "development_reads_and_writes_stay_in_local_storage",
            ])
            .env("VESPER_CREDENTIAL_TEST_CHILD", "1")
            .env("HOME", directory.path())
            .env("USERPROFILE", directory.path())
            .env("LOCALAPPDATA", directory.path())
            .env("XDG_DATA_HOME", directory.path())
            .env_remove("APP_LOCK_PASSWORD")
            .env_remove("TELEGRAM_API_ID")
            .env_remove("TELEGRAM_API_HASH")
            .env_remove("TELEGRAM_CHANNEL_USERNAME")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }

    assert!(matches!(
        vesper_credentials::app_lock().unwrap(),
        Stored::Missing
    ));
    assert!(matches!(
        vesper_credentials::telegram().unwrap(),
        Stored::Missing
    ));
    assert!(matches!(vesper_credentials::x().unwrap(), Stored::Missing));
    assert!(matches!(
        vesper_credentials::ugos_certificate().unwrap(),
        Stored::Missing
    ));

    vesper_credentials::save_app_lock("test-password").unwrap();
    vesper_credentials::save_telegram(TelegramCredentials {
        api_id: 1,
        api_hash: "0123456789abcdef0123456789abcdef".into(),
        channel_username: "test_channel".into(),
    })
    .unwrap();
    vesper_credentials::save_x(XCredentials {
        client_id: "test-client".into(),
        access_token: "test-access".into(),
        refresh_token: "test-refresh".into(),
        expires_at: 1,
    })
    .unwrap();
    vesper_credentials::save_ugos_certificate("test-fingerprint").unwrap();

    assert!(
        matches!(vesper_credentials::app_lock().unwrap(), Stored::Ready(value) if value.password == "test-password")
    );
    assert!(
        matches!(vesper_credentials::telegram().unwrap(), Stored::Ready(value) if value.api_id == 1)
    );
    assert!(
        matches!(vesper_credentials::x().unwrap(), Stored::Ready(value) if value.client_id == "test-client")
    );
    assert!(
        matches!(vesper_credentials::ugos_certificate().unwrap(), Stored::Ready(value) if value == "test-fingerprint")
    );
    vesper_credentials::delete_app_lock().unwrap();
    assert!(matches!(
        vesper_credentials::app_lock().unwrap(),
        Stored::Missing
    ));
}
