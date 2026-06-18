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
fn diff_resolves_git_revisions() {
    use std::fs;
    // A hermetic temp repo with two commits; diff the revisions (ADR 0007).
    let repo = std::env::temp_dir().join(format!("atlas_diff_rev_{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(&repo).expect("mk repo");

    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .output()
            .expect("run git")
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "atlas-test"]);

    fs::write(repo.join("m.py"), "def f(x):\n    pass\n").expect("write v1");
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "v1"]);

    fs::write(repo.join("m.py"), "def f(x, y):\n    pass\n").expect("write v2");
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "v2"]);

    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(["diff", "HEAD~1", "HEAD"])
        .current_dir(&repo)
        .output()
        .expect("run atlas");
    let _ = fs::remove_dir_all(&repo);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    assert!(stdout.contains("# atlas diff: HEAD~1 → HEAD"), "{stdout}");
    assert!(
        stdout.contains("~ function f: def f(x): → def f(x, y):"),
        "{stdout}"
    );
}

#[test]
fn diff_one_arg_compares_revision_to_working_tree() {
    use std::fs;
    // One-argument shorthand (#24): `atlas diff HEAD` diffs the committed HEAD
    // against the current (dirty) working tree.
    let repo = std::env::temp_dir().join(format!("atlas_diff_wt_{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(&repo).expect("mk repo");

    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .output()
            .expect("run git")
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "atlas-test"]);

    fs::write(repo.join("m.py"), "def f(x):\n    pass\n").expect("write committed");
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "v1"]);

    // Uncommitted working-tree edit — only visible to a working-tree diff.
    fs::write(repo.join("m.py"), "def f(x, y):\n    pass\n").expect("write working tree");

    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(["diff", "HEAD"])
        .current_dir(&repo)
        .output()
        .expect("run atlas");
    let _ = fs::remove_dir_all(&repo);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    assert!(
        stdout.contains("# atlas diff: HEAD → (working tree)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("~ function f: def f(x): → def f(x, y):"),
        "{stdout}"
    );
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

#[test]
fn diff_exit_code_gate_fails_on_breaking() {
    // old→new removes public `g` and changes public `f` — both breaking (#85).
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("diff")
        .arg(fixture("old"))
        .arg(fixture("new"))
        .arg("--exit-code")
        .output()
        .expect("failed to run atlas");
    assert_eq!(out.status.code(), Some(1), "breaking diff should exit 1");
}

#[test]
fn diff_exit_code_gate_passes_without_breaking() {
    // Identical trees → no breaking change → exit 0 even with --exit-code.
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg("diff")
        .arg(fixture("old"))
        .arg(fixture("old"))
        .arg("--exit-code")
        .output()
        .expect("failed to run atlas");
    assert!(out.status.success(), "no-breaking diff should exit 0");
}
