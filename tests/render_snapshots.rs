//! Golden snapshots + a determinism gate for the rendered map across all CLI
//! output formats (#82, #81). Drives the real binary on the committed
//! multi-language fixture tree and compares each format's output to a committed
//! golden under `tests/render_snapshots/`. Regenerate intentionally with
//! `UPDATE_SNAPSHOTS=1 cargo test --test render_snapshots` and review the diff
//! like code.

use std::path::PathBuf;
use std::process::Command;

const FIXTURE: &str = "tests/queries/fixtures";
const BUDGET: &str = "4096";

fn render(format: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            FIXTURE, "--budget", BUDGET, "--format", format, "--color", "never",
        ])
        .output()
        .expect("run atlas");
    assert!(
        out.status.success(),
        "atlas --format {format} exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8 output")
}

fn golden_path(ext: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/render_snapshots")
        .join(format!("fixtures.{ext}"))
}

fn assert_golden(format: &str, ext: &str) {
    let rendered = render(format);
    let path = golden_path(ext);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, &rendered).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} missing ({e}); regenerate with UPDATE_SNAPSHOTS=1 cargo test --test render_snapshots",
            path.display()
        )
    });
    assert_eq!(
        rendered,
        expected,
        "rendered {format} drifted vs {}; if intentional, regenerate and review",
        path.display()
    );
}

#[test]
fn markdown_matches_golden() {
    assert_golden("md", "md");
}

#[test]
fn json_matches_golden() {
    assert_golden("json", "json");
}

#[test]
fn xml_matches_golden() {
    assert_golden("xml", "xml");
}

#[test]
fn every_format_is_deterministic() {
    for format in ["md", "json", "xml"] {
        assert_eq!(
            render(format),
            render(format),
            "format {format} is not deterministic across runs"
        );
    }
}
