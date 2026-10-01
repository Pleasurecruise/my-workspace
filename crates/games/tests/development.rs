#![cfg(debug_assertions)]

use games::session::{Provider, Session};
use vault::Stored;

#[test]
fn steam_environment_overrides_file_without_hiding_invalid_configuration() {
    if let Ok(case) = std::env::var("VESPER_STEAM_TEST_CASE") {
        games::session::save(&Session::Steam {
            api_key: "file-key".into(),
            steam_id: "76561198000000000".into(),
        })
        .unwrap();
        let result = games::session::read(Provider::Steam);
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
