#![cfg(debug_assertions)]

use vault::Stored;

#[derive(serde::Deserialize, serde::Serialize)]
struct Sample {
    value: String,
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
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }

    assert!(matches!(
        vault::read("test-plain").unwrap(),
        Stored::Missing
    ));
    assert!(matches!(
        vault::read_json::<Sample>("test-json").unwrap(),
        Stored::Missing
    ));

    vault::save("test-plain", "test-value").unwrap();
    vault::save_json(
        "test-json",
        &Sample {
            value: "test-json-value".into(),
        },
    )
    .unwrap();

    assert!(
        matches!(vault::read("test-plain").unwrap(), Stored::Ready(value) if value == "test-value")
    );
    assert!(
        matches!(vault::read_json::<Sample>("test-json").unwrap(), Stored::Ready(sample) if sample.value == "test-json-value")
    );
    vault::delete("test-plain").unwrap();
    assert!(matches!(
        vault::read("test-plain").unwrap(),
        Stored::Missing
    ));
}
