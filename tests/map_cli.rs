//! End-to-end coverage for map-mode CLI behavior that is only visible at the
//! process boundary: stdout vs file output, stderr timings, and exit codes.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_repo(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-map-cli-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp repo");
    dir
}

fn write_python_repo(name: &str) -> PathBuf {
    let dir = temp_repo(name);
    fs::write(dir.join("app.py"), "def run(value):\n    return value\n").expect("write source");
    dir
}

fn run_atlas(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(args)
        .output()
        .expect("failed to run atlas")
}

#[test]
fn output_flag_writes_file_and_keeps_stdout_empty() {
    let repo = write_python_repo("output");
    let out_path = repo.join("atlas-map.md");
    let output = run_atlas(&[
        repo.to_str().unwrap(),
        "--budget",
        "200",
        "--color",
        "always",
        "--output",
        out_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "stdout should stay empty");
    let written = fs::read_to_string(&out_path).expect("read output file");
    assert!(written.contains("# atlas:"), "{written}");
    assert!(
        !written.contains("\u{1b}["),
        "file output must not be ANSI colored"
    );
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn for_agent_prepends_markdown_preamble() {
    let repo = write_python_repo("agent");
    let output = run_atlas(&[repo.to_str().unwrap(), "--budget", "200", "--for-agent"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.starts_with("> atlas agent note:"), "{stdout}");
    assert!(stdout.contains("\n# atlas:"), "{stdout}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn for_agent_rejects_structured_formats() {
    let repo = write_python_repo("agent-json");
    let output = run_atlas(&[repo.to_str().unwrap(), "--format", "json", "--for-agent"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("--for-agent is only supported"), "{stderr}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn timings_go_to_stderr_without_changing_map_stdout() {
    let repo = write_python_repo("timings");
    let output = run_atlas(&[repo.to_str().unwrap(), "--budget", "200", "--timings"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stdout.contains("# atlas:"), "{stdout}");
    assert!(stderr.contains("atlas timings: discover"), "{stderr}");
    assert!(stderr.contains("atlas timings: total"), "{stderr}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn empty_map_diagnostic_reports_seen_extensions() {
    let repo = temp_repo("empty");
    fs::write(repo.join("README.md"), "# docs only\n").expect("write docs");
    let output = run_atlas(&[repo.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("no supported source files"), "{stderr}");
    assert!(stderr.contains(".md (1)"), "{stderr}");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn unknown_lang_error_text_lists_tier2_extensions() {
    // The unknown-`--lang` message must advertise the Tier 2 extensions so a
    // user who tries an unsupported language sees Go/Java/C/C++ are covered (#35).
    let repo = write_python_repo("unknown-lang");
    let output = run_atlas(&[repo.to_str().unwrap(), "--lang", "cobol"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("unknown --lang value"), "{stderr}");
    for ext in ["go", "java", "c", "cpp", "hpp"] {
        assert!(
            stderr.contains(ext),
            "error text should list Tier 2 extension {ext:?}: {stderr}"
        );
    }
    assert!(stderr.contains("Go, Java"), "{stderr}");
    let _ = fs::remove_dir_all(repo);
}
