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
    let output = Command::new(env!("CARGO_BIN_EXE_vesper"))
        .arg("help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for command in [
        "memo get",
        "knowledge page",
        "moment query",
        "moment get",
        "status [source]",
        "--stdin",
    ] {
        assert!(help.contains(command), "missing command: {command}");
    }
}
