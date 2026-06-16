//! Query tests, auto-run by the post-edit hook (`cargo test query_`)
//! whenever a `queries/**/*.scm` file changes.
//!
//! Two layers:
//! - contract tests (all Tier 1 languages): the tags.scm exists and uses
//!   only capture names src/parse.rs understands;
//! - snapshot tests (wired grammars — Python since M0): parse the fixture,
//!   run extraction, and compare against the committed snapshot in
//!   tests/queries/snapshots/. Regenerate intentionally with
//!   `UPDATE_SNAPSHOTS=1 cargo test query_` and review the diff like code.

const CAPTURE_PREFIXES: &[&str] = &["@definition.", "@reference.", "@name"];

const DEFINITION_KINDS: &[&str] = &[
    "function",
    "method",
    "class",
    "interface",
    "enum",
    "type",
    "constant",
    "module",
];

fn assert_tags_contract(lang: &str) {
    let path = format!("queries/{lang}/tags.scm");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} must exist: {e}"));
    assert!(!src.trim().is_empty(), "{path} is empty");
    assert!(
        src.contains("@definition."),
        "{path} defines no @definition captures"
    );

    // Skip comment lines; every capture used must come from the contract.
    for line in src.lines().filter(|l| !l.trim_start().starts_with(';')) {
        for token in line.split_whitespace().filter(|t| t.starts_with('@')) {
            let token = token.trim_end_matches(')');
            assert!(
                CAPTURE_PREFIXES.iter().any(|p| token.starts_with(p)),
                "{path}: capture {token} is outside the contract documented in the file header"
            );
            if let Some(kind) = token.strip_prefix("@definition.") {
                assert!(
                    DEFINITION_KINDS.contains(&kind),
                    "{path}: unknown definition kind {kind:?} (allowed: {DEFINITION_KINDS:?})"
                );
            }
        }
    }
}

#[test]
fn query_python_tags_contract() {
    assert_tags_contract("python");
}

#[test]
fn query_typescript_tags_contract() {
    assert_tags_contract("typescript");
}

#[test]
fn query_rust_tags_contract() {
    assert_tags_contract("rust");
}

/// Parse a committed fixture, render its extraction, and diff against the
/// committed snapshot. `UPDATE_SNAPSHOTS=1` regenerates (review the diff like
/// code). Covers every wired Tier 1 grammar so a query change in any language
/// fails loudly here.
fn assert_fixture_snapshot(stem: &str, ext: &str, lang: repomap::lang::Language) {
    let rel = format!("{stem}.{ext}");
    let fixture = format!("tests/queries/fixtures/{rel}");
    let snapshot_path = format!("tests/queries/snapshots/{stem}.snap");

    let file = repomap::discover::SourceFile {
        path: std::path::PathBuf::from(&fixture),
        rel: rel.clone(),
        lang,
    };
    let parsed = repomap::parse::parse_file(&file)
        .unwrap_or_else(|| panic!("{fixture} must parse — it is our own fixture"));

    let mut rendered = String::new();
    rendered.push_str(&format!(
        "# extraction snapshot for queries/{}/tags.scm\n",
        lang.name()
    ));
    rendered.push_str("# regenerate: UPDATE_SNAPSHOTS=1 cargo test query_  (review the diff!)\n");
    for s in &parsed.symbols {
        rendered.push_str(&format!(
            "L{:<3} {:<9} {:<7} {:<15} :: {}\n",
            s.line,
            format!("{:?}", s.kind),
            format!("{:?}", s.visibility),
            s.name,
            s.signature
        ));
    }
    rendered.push_str(&format!("imports: {}\n", parsed.imports.join(", ")));
    rendered.push_str(&format!("references: {}\n", parsed.references.join(", ")));

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all("tests/queries/snapshots").expect("create snapshots dir");
        std::fs::write(&snapshot_path, &rendered).expect("write snapshot");
        return;
    }
    let expected = std::fs::read_to_string(&snapshot_path).unwrap_or_else(|e| {
        panic!("{snapshot_path} missing ({e}); run UPDATE_SNAPSHOTS=1 cargo test query_")
    });
    assert_eq!(
        rendered, expected,
        "extraction changed vs {snapshot_path}; if intentional, regenerate with UPDATE_SNAPSHOTS=1 and review the diff"
    );
}

#[test]
fn query_python_fixture_snapshot() {
    assert_fixture_snapshot("python", "py", repomap::lang::Language::Python);
}

#[test]
fn query_typescript_fixture_snapshot() {
    assert_fixture_snapshot("typescript", "ts", repomap::lang::Language::TypeScript);
}

#[test]
fn query_rust_fixture_snapshot() {
    assert_fixture_snapshot("rust", "rs", repomap::lang::Language::Rust);
}

#[test]
fn query_fixtures_exist_for_every_tier1_language() {
    for fixture in ["python.py", "typescript.ts", "rust.rs"] {
        let path = format!("tests/queries/fixtures/{fixture}");
        assert!(
            std::path::Path::new(&path).exists(),
            "missing fixture {path} — every tags.scm needs a fixture exercising its constructs"
        );
    }
}
