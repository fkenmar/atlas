//! End-to-end coverage for the git-style `atlas serve --mcp` route.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn serve_without_mcp_exits_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("serve")
        .output()
        .expect("failed to run atlas");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(
        stderr.contains("serve currently supports only --mcp"),
        "{stderr}"
    );
}

#[test]
fn serve_mcp_handles_initialize_over_stdio() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(["serve", "--mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn atlas serve --mcp");

    {
        let stdin = child.stdin.as_mut().expect("stdin should be piped");
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize"}}"#)
            .expect("write request");
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait for server");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains(r#""jsonrpc":"2.0""#), "{stdout}");
    assert!(stdout.contains(r#""serverInfo""#), "{stdout}");
    assert!(stdout.contains(r#""name":"atlas""#), "{stdout}");
}
