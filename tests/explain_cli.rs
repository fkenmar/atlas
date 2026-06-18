//! End-to-end coverage of `atlas explain <path>` (#94): the rank-explanation
//! debug command, driven against the committed fixture tree.

use std::process::Command;

fn explain(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("explain")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run atlas explain")
}

#[test]
fn explain_reports_rank_signals() {
    let out = explain(&["python.py", "--root", "tests/queries/fixtures"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    assert!(
        stdout.contains("rank explanation for python.py"),
        "{stdout}"
    );
    assert!(stdout.contains("rank:      #"), "{stdout}");
    assert!(stdout.contains("score:"), "{stdout}");
    assert!(stdout.contains("importers:"), "{stdout}");
    assert!(stdout.contains("imports:"), "{stdout}");
    assert!(stdout.contains("focus:"), "{stdout}");
}

#[test]
fn explain_unknown_file_errors() {
    let out = explain(&["does-not-exist.py", "--root", "tests/queries/fixtures"]);
    assert!(!out.status.success());
    assert_ne!(out.status.code(), Some(0));
}
