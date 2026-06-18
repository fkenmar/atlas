//! Parser robustness (#83, FR-12): malformed, truncated, and adversarial source
//! must be parsed without panicking — `parse_file` always returns (`Some`/`None`),
//! never aborts. Feeds a battery of pathological inputs through every wired
//! grammar. (A true continuous fuzzer via cargo-fuzz/proptest would add a
//! dependency; this is the dependency-free robustness battery.)

use std::fs;
use std::path::PathBuf;

use atlas::discover::SourceFile;
use atlas::lang::Language;
use atlas::parse::parse_file;

/// Every file extension wired to a grammar.
const EXTENSIONS: &[&str] = &["py", "ts", "rs", "go", "java", "c", "cpp"];

/// Adversarial source fragments — none should ever panic the parser.
fn nasty_inputs() -> Vec<String> {
    let mut inputs: Vec<String> = vec![
        String::new(),                         // empty
        " ".repeat(10_000),                    // only whitespace
        "\u{0}\u{1}\u{2}\u{FFFE}".to_string(), // control + noncharacter
        "\"unterminated string".to_string(),
        "(".repeat(5_000), // unbalanced, deeply nested open
        ")".repeat(5_000), // unbalanced close
        "{".repeat(5_000),
        "def ".to_string(), // truncated declaration
        "def f(".to_string(),
        "class ".to_string(),
        "fn f<".to_string(),
        "struct {".to_string(),
        "func (".to_string(),
        "a".repeat(100_000),  // one enormous identifier
        "x\n".repeat(50_000), // very many lines
        "/* unterminated comment".to_string(),
        "🦀".repeat(5_000), // multi-byte unicode
        "import\nimport\nimport".to_string(),
        "\t\r\n\t\r\n".repeat(1_000), // mixed whitespace/newlines
        "namespace a { class B { void c( } }".to_string(),
    ];
    // Plus prefix-truncations of a valid-ish snippet (catches mid-token aborts).
    let valid = "pub fn helper(x: i32) -> i32 { let y = x + 1; y }\n";
    for end in (0..valid.len()).step_by(3) {
        inputs.push(valid[..end].to_string());
    }
    inputs
}

#[test]
fn parser_never_panics_on_malformed_input() {
    let dir = std::env::temp_dir().join(format!("atlas-fuzz-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");

    let inputs = nasty_inputs();
    let mut parsed_calls = 0usize;
    for (i, input) in inputs.iter().enumerate() {
        for ext in EXTENSIONS {
            let path: PathBuf = dir.join(format!("case_{i}.{ext}"));
            fs::write(&path, input).expect("write case");
            let lang = Language::from_extension(ext).expect("wired extension");
            let file = SourceFile {
                path: path.clone(),
                rel: format!("case_{i}.{ext}"),
                lang,
            };
            // The assertion is simply that this returns — a panic fails the test
            // (FR-12: unparseable input is skipped, never a crash).
            let _ = parse_file(&file);
            parsed_calls += 1;
        }
    }

    let _ = fs::remove_dir_all(&dir);
    assert!(parsed_calls > 0, "no inputs exercised");
}
