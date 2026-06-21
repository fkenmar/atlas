//! Conformance between the committed JSON Schemas (`schemas/`) and what the JSON
//! renderers actually emit (#57, #58). We don't pull a full JSON-Schema
//! validator (that would be a new dependency); instead we render real output and
//! assert the structure-defining parts agree: the top-level shape and the
//! controlled vocabularies (symbol kinds, visibility, severity, budget detail).
//! Those enums are the most drift-prone surface, and they live in code
//! (`kind_name`/`visibility_name`/`detail_name`/`Severity::as_str`) separately
//! from the schema, so this catches a renderer change that forgets the schema.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_schema(name: &str) -> Value {
    let path = repo_root().join("schemas").join(name);
    let text = fs::read_to_string(&path).expect("read schema file");
    let schema: Value = serde_json::from_str(&text).expect("schema is valid JSON");
    assert_eq!(
        schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
        "{name} should declare the draft 2020-12 dialect"
    );
    assert_eq!(schema["type"], "object", "{name} top-level type");
    schema
}

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-schema-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn run_atlas(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_atlas"))
        .args(args)
        .output()
        .expect("run atlas");
    assert!(
        out.status.success(),
        "atlas {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8 stdout")
}

/// Collect every string value stored under `key` anywhere in the JSON tree.
fn collect_strings<'a>(value: &'a Value, key: &str, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if k == key {
                    if let Some(s) = v.as_str() {
                        out.push(s);
                    }
                }
                collect_strings(v, key, out);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_strings(v, key, out);
            }
        }
        _ => {}
    }
}

fn enum_set<'a>(schema: &'a Value, pointer: &str) -> Vec<&'a str> {
    schema
        .pointer(pointer)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("schema enum at {pointer}"))
        .iter()
        .filter_map(Value::as_str)
        .collect()
}

fn assert_subset(observed: &[&str], allowed: &[&str], what: &str) {
    for v in observed {
        assert!(
            allowed.contains(v),
            "{what} value {v:?} emitted by the renderer is not in the schema enum {allowed:?}"
        );
    }
}

fn require_keys(obj: &Value, keys: &[&str], what: &str) {
    let map = obj
        .as_object()
        .unwrap_or_else(|| panic!("{what} is an object"));
    for k in keys {
        assert!(map.contains_key(*k), "{what} missing required key {k:?}");
    }
}

#[test]
fn map_json_conforms_to_schema() {
    let schema = load_schema("atlas-map.schema.json");

    // A small multi-kind, mixed-visibility repo so the output exercises several
    // symbol kinds and both visibilities.
    let repo = temp_dir("map");
    fs::write(
        repo.join("svc.py"),
        "API_VERSION = \"1.0\"\n\
         \n\
         def run(x: int) -> int:\n    return x\n\
         \n\
         def _private(x):\n    return x\n\
         \n\
         class Service:\n\
         \x20   name: str\n\
         \x20   def total(self) -> int:\n        return 0\n",
    )
    .expect("write source");

    let out = run_atlas(&[
        repo.to_str().unwrap(),
        "--budget",
        "2000",
        "--format",
        "json",
    ]);
    let doc: Value = serde_json::from_str(&out).expect("map JSON parses");

    // Top-level shape (mirrors the schema's required + additionalProperties:false).
    require_keys(
        &doc,
        &[
            "version",
            "repo",
            "budget",
            "files",
            "collapsed",
            "symbol_index",
            "skipped_files",
            "unwired_files",
        ],
        "map",
    );
    let allowed_top: Vec<&str> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for k in doc.as_object().unwrap().keys() {
        assert!(
            allowed_top.contains(&k.as_str()),
            "map has key {k:?} not declared in schema (additionalProperties:false)"
        );
    }

    // Controlled vocabularies.
    let kinds = enum_set(&schema, "/$defs/symbolKind/enum");
    let vis = enum_set(&schema, "/$defs/visibility/enum");
    let detail = enum_set(&schema, "/properties/budget/properties/detail/enum");

    let mut observed_kinds = Vec::new();
    collect_strings(&doc, "kind", &mut observed_kinds);
    assert!(!observed_kinds.is_empty(), "fixture should yield symbols");
    assert_subset(&observed_kinds, &kinds, "symbol kind");

    let mut observed_vis = Vec::new();
    collect_strings(&doc, "visibility", &mut observed_vis);
    assert!(
        observed_vis.contains(&"private") && observed_vis.contains(&"public"),
        "fixture should exercise both visibilities: {observed_vis:?}"
    );
    assert_subset(&observed_vis, &vis, "visibility");

    assert_subset(
        &[doc["budget"]["detail"].as_str().expect("detail string")],
        &detail,
        "budget detail",
    );

    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn diff_json_conforms_to_schema() {
    let schema = load_schema("atlas-diff.schema.json");

    // old -> new: one changed signature + one added file, so the diff has
    // changed_files and added_files with severities.
    let base = temp_dir("diff");
    let old = base.join("old");
    let new = base.join("new");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&new).unwrap();
    fs::write(old.join("mod.py"), "def run(x):\n    return x\n").unwrap();
    fs::write(new.join("mod.py"), "def run(x, y):\n    return x + y\n").unwrap();
    fs::write(new.join("added.py"), "def helper():\n    return 1\n").unwrap();

    let out = run_atlas(&[
        "diff",
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let doc: Value = serde_json::from_str(&out).expect("diff JSON parses");

    require_keys(
        &doc,
        &[
            "version",
            "old",
            "new",
            "moved_files",
            "added_files",
            "removed_files",
            "changed_files",
            "skipped",
        ],
        "diff",
    );
    let allowed_top: Vec<&str> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for k in doc.as_object().unwrap().keys() {
        assert!(
            allowed_top.contains(&k.as_str()),
            "diff has key {k:?} not declared in schema (additionalProperties:false)"
        );
    }

    // Every severity the renderer emits must be in the schema's enum.
    let severities = enum_set(&schema, "/$defs/severity/enum");
    let mut observed = Vec::new();
    collect_strings(&doc, "severity", &mut observed);
    assert!(
        !observed.is_empty(),
        "the changed+added fixture should produce at least one severity"
    );
    assert_subset(&observed, &severities, "severity");

    // Sanity: the added file shows up.
    let added = doc["added_files"].as_array().expect("added_files array");
    assert!(
        added.iter().any(|f| f["path"] == "added.py"),
        "added.py should be reported as an added file"
    );

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn schema_versions_match_renderers() {
    // The schemas claim to describe version 1; keep that honest against the
    // constants the renderers embed. If a renderer bumps its version, the schema
    // (and this assertion) must be updated deliberately.
    let map = load_schema("atlas-map.schema.json");
    let diff = load_schema("atlas-diff.schema.json");
    assert!(map["description"].as_str().unwrap().contains("version 1"));
    assert!(diff["description"].as_str().unwrap().contains("version 1"));

    let map_out = {
        let repo = temp_dir("ver");
        fs::write(repo.join("a.py"), "def f():\n    return 1\n").unwrap();
        let out = run_atlas(&[repo.to_str().unwrap(), "--format", "json"]);
        let _ = fs::remove_dir_all(&repo);
        serde_json::from_str::<Value>(&out).unwrap()
    };
    assert_eq!(map_out["version"], 1, "map schema describes version 1");
}
