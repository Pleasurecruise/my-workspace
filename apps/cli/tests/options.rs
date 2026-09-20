use std::process::Command;

#[test]
fn version_flags_print_package_version_without_initializing_logging() {
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_vesper"))
            .arg(flag)
            .env("RUST_LOG", "[invalid")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("vesper {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn help_does_not_initialize_runtime() {
    for arguments in [
        vec![],
        vec!["help"],
        vec!["--help"],
        vec!["-h"],
        vec!["memo", "list", "--help"],
        vec!["help", "memo", "list"],
        vec!["todo", "notion", "connect", "-h"],
        vec!["moment", "upload-photo", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_vesper"))
            .args(arguments)
            .env("RUST_LOG", "[invalid")
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("Usage: vesper"));
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn usage_errors_precede_runtime_initialization() {
    for arguments in [
        vec!["unknown"],
        vec!["memo", "lsit"],
        vec!["memo", "get"],
        vec!["memo", "list", "--unknown"],
        vec!["memo", "create", "--stdin", "--file", "note.md"],
        vec!["game", "notes", "unknown"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_vesper"))
            .args(arguments)
            .env("RUST_LOG", "[invalid")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("error:"));
        assert!(!error.contains("initialize logging"));
    }
}
