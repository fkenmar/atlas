//! End-to-end coverage for `atlas doctor` (#47): the diagnostic must report
//! version + per-language file counts on a supported repo, name an
//! unsupported language, and handle an empty repo — always exiting 0 on a valid
//! path (it reports problems, it doesn't fail on them).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_repo(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-doctor-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp repo");
    dir
}

fn run_doctor(path: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(["doctor", path])
        .output()
        .expect("run atlas doctor")
}

#[test]
fn doctor_reports_version_and_per_language_counts() {
    let repo = temp_repo("supported");
    fs::write(repo.join("app.py"), "def run():\n    return 1\n").unwrap();
    fs::write(repo.join("lib.rs"), "pub fn helper() {}\n").unwrap();

    let output = run_doctor(repo.to_str().unwrap());
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");

    assert!(stdout.contains("atlas doctor"), "{stdout}");
    assert!(stdout.contains("version:"), "{stdout}");
    assert!(stdout.contains("2 found"), "{stdout}");
    assert!(stdout.contains("python: 1"), "{stdout}");
    assert!(stdout.contains("rust: 1"), "{stdout}");
    assert!(stdout.contains("cache:"), "{stdout}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn doctor_names_unsupported_language_on_empty_map() {
    let repo = temp_repo("ruby");
    fs::write(repo.join("app.rb"), "def run\n  1\nend\n").unwrap();

    let output = run_doctor(repo.to_str().unwrap());
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("0 found"), "{stdout}");
    assert!(stdout.contains(".rb"), "{stdout}");
    assert!(stdout.contains("Ruby"), "{stdout}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn doctor_handles_empty_repo() {
    let repo = temp_repo("empty");
    let output = run_doctor(repo.to_str().unwrap());
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("0 found"), "{stdout}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn doctor_bad_path_is_usage_error() {
    let missing = temp_repo("missing");
    let _ = fs::remove_dir_all(&missing);
    let output = run_doctor(missing.to_str().unwrap());
    assert_eq!(output.status.code(), Some(2));
}
