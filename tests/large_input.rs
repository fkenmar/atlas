//! Perf/scale guard (#84): a pathological large/generated source file must map
//! without hanging or blowing the token budget — the degradation ladder has to
//! hold. Drives the real binary on a generated repo and asserts it completes,
//! produces a map, and respects the requested budget.

use std::fs;
use std::process::Command;

#[test]
fn large_generated_file_completes_within_budget() {
    let dir = std::env::temp_dir().join(format!("atlas-large-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");

    // A single ~generated file: thousands of functions across a few modules.
    let mut huge = String::with_capacity(400_000);
    for i in 0..4_000u32 {
        huge.push_str(&format!(
            "pub fn generated_symbol_{i}(arg_{i}: u32) -> u32 {{ arg_{i} + {i} }}\n"
        ));
    }
    fs::write(dir.join("generated.rs"), &huge).expect("write generated");
    // Plus a smaller hand-written module so ranking has something to compare.
    fs::write(
        dir.join("lib.rs"),
        "pub fn entry() -> u32 { generated_symbol_0(1) }\n",
    )
    .expect("write lib");

    let budget = 2048usize;
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .arg(&dir)
        .args(["--budget", &budget.to_string()])
        .output()
        .expect("run atlas on a large repo");
    let _ = fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "atlas failed on a large input: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    // It produced a map header.
    assert!(stdout.contains("# atlas:"), "{stdout}");

    // The degradation ladder kept the rendered output within the budget (with a
    // small structural-overhead margin) rather than dumping 4,000 signatures.
    let rendered = stdout
        .split("rendered ")
        .nth(1)
        .and_then(|s| s.split(" tok").next())
        .and_then(|n| n.trim().parse::<usize>().ok())
        .unwrap_or_else(|| panic!("could not find 'rendered N tok' in header: {stdout}"));
    assert!(
        rendered <= budget + budget / 10,
        "rendered {rendered} tok exceeds budget {budget} (+10% margin) — degradation ladder failed"
    );
}
