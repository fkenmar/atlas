//! End-to-end coverage of the `atlas diff` command (issue #12): drives the real
//! binary against the committed fixture trees under `tests/diff_fixture/`,
//! exercising the run_diff/discover_tree wiring and its exit codes — the glue
//! the in-crate unit tests can't reach.

use std::path::PathBuf;
use std::process::Command;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/diff_fixture")
        .join(rel)
}

fn run_diff(old: &str, new: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("diff")
        .arg(fixture(old))
        .arg(fixture(new))
        .output()
        .expect("failed to run atlas")
}

#[test]
fn diff_reports_structural_delta() {
    let out = run_diff("old", "new");
    assert!(out.status.success(), "diff should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("utf-8 output");

    // Added file (the class + its method).
    assert!(stdout.contains("## Added files"), "{stdout}");
    assert!(
        stdout.contains("+ added.py (python, 2 symbol(s))"),
        "{stdout}"
    );
    // Changed file: signature change, an added and a removed symbol.
    assert!(stdout.contains("### mod.py"), "{stdout}");
    assert!(
        stdout.contains("~ function f: def f(x): → def f(x, y):"),
        "{stdout}"
    );
    assert!(stdout.contains("+ def h():"), "{stdout}");
    assert!(stdout.contains("- def g():"), "{stdout}");
    // Import-edge delta.
    assert!(stdout.contains("imports +: b"), "{stdout}");
    assert!(stdout.contains("imports -: a"), "{stdout}");
}

#[test]
fn diff_json_format_emits_structured_delta() {
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("diff")
        .arg(fixture("old"))
        .arg(fixture("new"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("failed to run atlas");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).expect("utf-8 output");
    assert!(stdout.contains("\"added_files\""), "{stdout}");
    assert!(stdout.contains("\"changed_files\""), "{stdout}");
    assert!(stdout.contains("\"path\": \"added.py\""), "{stdout}");
    assert!(stdout.contains("\"new_sig\": \"def f(x, y):\""), "{stdout}");
}

#[test]
fn diff_is_deterministic_end_to_end() {
    let a = run_diff("old", "new");
    let b = run_diff("old", "new");
    assert_eq!(a.stdout, b.stdout);
}

#[test]
fn diff_nonexistent_path_exits_2() {
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("diff")
        .arg(fixture("old"))
        .arg(fixture("does-not-exist"))
        .output()
        .expect("failed to run atlas");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn diff_file_instead_of_dir_exits_2() {
    // Passing a file where a directory is expected hits discover_tree's
    // not-a-directory branch via the shared canonicalize_root helper.
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("diff")
        .arg(fixture("old/mod.py"))
        .arg(fixture("new"))
        .output()
        .expect("failed to run atlas");
    assert_eq!(out.status.code(), Some(2));
}
