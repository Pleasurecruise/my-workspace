use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn reads_json_from_stdin_before_consumer_requests() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vesper"))
        .args(["knowledge", "create", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"not-json\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("invalid knowledge create JSON")
    );
}

#[test]
fn help_lists_consumer_queries() {
    for (domain, commands) in [
        ("memo", vec!["get", "list", "page", "patch", "import-x"]),
        ("knowledge", vec!["page", "get", "update-documents"]),
        ("moment", vec!["query", "get", "upload-photo"]),
        ("todo", vec!["check-ins", "undo-check-in", "notion"]),
        ("ledger", vec!["create", "list"]),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_vesper"))
            .args([domain, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        for command in commands {
            assert!(
                help.contains(command),
                "missing command: {domain} {command}"
            );
        }
    }
}
