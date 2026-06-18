//! End-to-end coverage of `atlas cache info|clean` (#80): populate a cache by
//! mapping a temp repo, then inspect and clear it through the real binary.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn atlas(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(args)
        .output()
        .expect("run atlas")
}

#[test]
fn cache_info_and_clean_roundtrip() {
    let repo = std::env::temp_dir().join(format!("atlas-cache-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(&repo).expect("mk repo");
    fs::write(repo.join("m.py"), "def f(x):\n    return x\n").expect("write source");
    let repo_str = repo.to_string_lossy().to_string();
    let cache_file: PathBuf = repo.join(".atlas").join("cache");

    // Populate the cache by mapping the repo.
    assert!(atlas(&[&repo_str, "--budget", "1000"]).status.success());
    assert!(cache_file.is_file(), "map run should write the cache");

    // info reports a present cache with entries.
    let info = atlas(&["cache", "info", &repo_str]);
    assert!(info.status.success());
    let info_out = String::from_utf8(info.stdout).expect("utf-8");
    assert!(info_out.contains("status:        present"), "{info_out}");
    assert!(info_out.contains("entries:"), "{info_out}");
    assert!(info_out.contains("cache version:"), "{info_out}");

    // clean without --force refuses and exits 2, leaving the cache in place.
    let refused = atlas(&["cache", "clean", &repo_str]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(
        cache_file.is_file(),
        "clean without --force must not delete"
    );

    // clean --force removes it.
    let cleaned = atlas(&["cache", "clean", &repo_str, "--force"]);
    assert!(cleaned.status.success());
    assert!(
        !cache_file.is_file(),
        "clean --force should remove the cache"
    );

    // info now reports absent.
    let info2 = atlas(&["cache", "info", &repo_str]);
    let info2_out = String::from_utf8(info2.stdout).expect("utf-8");
    assert!(info2_out.contains("status:        absent"), "{info2_out}");

    let _ = fs::remove_dir_all(&repo);
}
