//! Discover-stage integration test against the committed fixture tree in
//! tests/discover_fixture/ (vendored dirs and hidden dirs excluded, output
//! sorted — NFR-4).

use atlas::discover::discover;
use atlas::lang::Language;

#[test]
fn walks_fixture_tree_with_exclusions_and_sorted_output() {
    let root = std::path::Path::new("tests/discover_fixture");
    assert!(root.is_dir(), "fixture tree missing");

    let files = discover(root);
    let rels: Vec<&str> = files.iter().map(|f| f.rel.as_str()).collect();

    assert_eq!(
        rels,
        vec!["app.py", "sub/util.py"],
        "expected exactly the two non-vendored, non-hidden .py files, sorted"
    );
    assert!(files.iter().all(|f| f.lang == Language::Python));
}
