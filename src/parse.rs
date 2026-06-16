//! Stage 2 — parse: per-file tree-sitter parse extracting declarations via
//! the embedded `queries/<lang>/tags.scm` query files.
//!
//! Serial in M0; rayon parallelism lands in M1. Unparseable files are
//! skipped and counted, never a panic (FR-12). Files whose language grammar
//! isn't wired yet (TS/JS, Rust until M1) are counted separately.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::cache::Cache;
use crate::discover::SourceFile;
use crate::lang::Language;

/// A declaration extracted from one file.
#[derive(Clone, Debug, bincode::Encode, bincode::Decode)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// The source line containing the declaration's name, trimmed. M0 keeps
    /// only the first line of multi-line signatures.
    pub signature: String,
    /// 1-based line of the declaration name.
    pub line: usize,
    pub visibility: Visibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Enum,
    TypeAlias,
    Constant,
    Module,
    // Field kept last so existing bincode variant indices are unchanged.
    Field,
}

impl SymbolKind {
    fn from_capture(kind: &str) -> Option<SymbolKind> {
        match kind {
            "function" => Some(SymbolKind::Function),
            "method" => Some(SymbolKind::Method),
            "class" => Some(SymbolKind::Class),
            "interface" => Some(SymbolKind::Interface),
            "enum" => Some(SymbolKind::Enum),
            "type" => Some(SymbolKind::TypeAlias),
            "constant" => Some(SymbolKind::Constant),
            "module" => Some(SymbolKind::Module),
            "field" => Some(SymbolKind::Field),
            _ => None,
        }
    }

    /// Dedup priority: the same declaration can match several query
    /// patterns (a method also matches the bare function pattern); the
    /// more specific kind wins.
    fn priority(self) -> u8 {
        match self {
            SymbolKind::Method => 2,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub enum Visibility {
    Public,
    Private,
}

/// Parse result for one file: declarations plus outgoing edges-to-be.
#[derive(Clone, Debug, Default, bincode::Encode, bincode::Decode)]
pub struct ParsedFile {
    /// Sorted by (line, name) — deterministic (NFR-4).
    pub symbols: Vec<Symbol>,
    /// Raw import targets (module paths), resolved to files in link (M1).
    pub imports: Vec<String>,
    /// Call-site reference names, resolved in link (M1).
    pub references: Vec<String>,
    /// Source line count, for the map header's LOC figure.
    pub lines: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ParseStats {
    pub parsed_files: usize,
    /// Unreadable, non-UTF-8, or tree-sitter-rejected files (FR-12).
    pub skipped_files: usize,
    /// Files in Tier 1 languages whose grammar isn't wired yet (M1).
    pub unwired_files: usize,
    pub total_lines: usize,
}

pub struct ParseOutcome {
    /// (file, parse result), in discover's sorted order.
    pub files: Vec<(SourceFile, ParsedFile)>,
    pub stats: ParseStats,
}

/// Parse every discovered file, partitioning into parsed / skipped /
/// not-yet-wired (FR-12: nothing here ever panics on input). Uncached — see
/// [`parse_all_cached`] for the warm-path variant the CLI uses.
pub fn parse_all(files: Vec<SourceFile>) -> ParseOutcome {
    parse_all_cached(files, &mut Cache::disabled())
}

/// Parse with the incremental cache (FR-6): an unchanged file (same content
/// hash) reuses its stored parse instead of re-running tree-sitter. The hash
/// is computed from the single read this function already performs, so the
/// warm path never reads a file twice.
pub fn parse_all_cached(files: Vec<SourceFile>, cache: &mut Cache) -> ParseOutcome {
    let mut out = ParseOutcome {
        files: Vec::new(),
        stats: ParseStats::default(),
    };
    for file in files {
        if file.lang.grammar().is_none() {
            out.stats.unwired_files += 1;
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&file.path) else {
            out.stats.skipped_files += 1;
            continue;
        };
        let hash = crate::cache::content_hash(&source);
        let parsed = match cache.get(&file.rel, hash) {
            Some(parsed) => parsed,
            None => match parse_source(&file, &source) {
                Some(parsed) => {
                    cache.insert(&file.rel, hash, &parsed);
                    parsed
                }
                None => {
                    out.stats.skipped_files += 1;
                    continue;
                }
            },
        };
        out.stats.parsed_files += 1;
        out.stats.total_lines += parsed.lines;
        out.files.push((file, parsed));
    }
    out
}

/// Parse one file with the tree-sitter grammar for its language.
/// Returns `None` for unreadable/unparseable files; the caller counts them
/// and the renderer reports the count in a footer line (FR-12).
pub fn parse_file(file: &SourceFile) -> Option<ParsedFile> {
    let source = std::fs::read_to_string(&file.path).ok()?;
    parse_source(file, &source)
}

/// Parse already-in-memory source. Split out so the cache path
/// ([`parse_all_cached`]) can hash the content and parse from a single read.
fn parse_source(file: &SourceFile, source: &str) -> Option<ParsedFile> {
    let grammar = file.lang.grammar()?;
    let query = compiled_query(file.lang)?;

    let mut parser = Parser::new();
    parser.set_language(&grammar).ok()?;
    let tree = parser.parse(source, None)?;

    let lines: Vec<&str> = source.lines().collect();
    let capture_names = query.capture_names();

    // Rust test scaffolding (functions/helpers/the `mod tests` symbol itself)
    // is noise in the structural map and crowds out the real API surface.
    // Collect the row ranges of every `#[cfg(test)]` / `tests`-named module so
    // their declarations can be dropped. Empty for non-Rust.
    let test_ranges = test_module_ranges(file.lang, &tree, source.as_bytes());

    // Keyed by (line of name, name) so duplicate pattern matches collapse;
    // BTreeMap keeps the deterministic (line, name) order for free.
    let mut symbols: BTreeMap<(usize, String), Symbol> = BTreeMap::new();
    let mut imports: BTreeSet<String> = BTreeSet::new();
    let mut references: BTreeSet<String> = BTreeSet::new();

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        let mut name_text: Option<&str> = None;
        let mut name_row: usize = 0;
        let mut definition: Option<SymbolKind> = None;
        let mut reference: Option<&str> = None;
        let mut outer_text: Option<&str> = None;
        // Row of the outer (definition/reference) node, used to suppress
        // imports/calls that originate inside a test module — the `@name`
        // row alone is the right anchor for symbols and bare calls, but the
        // outer node anchors whole-node captures like Rust `use` items.
        let mut outer_row: usize = 0;

        for capture in m.captures {
            let capture_name = capture_names[capture.index as usize];
            let text = source.get(capture.node.byte_range()).unwrap_or("");
            if capture_name == "name" {
                name_text = Some(text);
                name_row = capture.node.start_position().row;
            } else if let Some(kind) = capture_name.strip_prefix("definition.") {
                definition = SymbolKind::from_capture(kind);
                outer_text = Some(text);
                outer_row = capture.node.start_position().row;
            } else if let Some(kind) = capture_name.strip_prefix("reference.") {
                reference = Some(kind);
                outer_text = Some(text);
                outer_row = capture.node.start_position().row;
            }
        }

        // Suppress anything originating inside a Rust test module — the test
        // fns, helpers, the `mod tests` symbol itself, and the spurious import
        // and call edges that test scaffolding would otherwise add to the
        // graph. Empty `test_ranges` (non-Rust) makes this a no-op.
        let in_test = |row: usize| test_ranges.iter().any(|r| r.contains(&row));
        if in_test(name_row) || in_test(outer_row) {
            continue;
        }

        match (definition, reference) {
            (Some(kind), _) => {
                let Some(name) = name_text else { continue };
                // The constant pattern matches every module-level assignment;
                // keep only conventional UPPER_SNAKE constants (documented in
                // the tags.scm files).
                if kind == SymbolKind::Constant && !is_const_name(name) {
                    continue;
                }
                let signature = lines
                    .get(name_row)
                    .map(|l| l.trim().to_string())
                    .unwrap_or_default();
                let visibility = visibility_of(file.lang, &signature, name);
                let symbol = Symbol {
                    name: name.to_string(),
                    kind,
                    signature,
                    line: name_row + 1,
                    visibility,
                };
                symbols
                    .entry((name_row, name.to_string()))
                    .and_modify(|existing| {
                        if kind.priority() > existing.kind.priority() {
                            *existing = symbol.clone();
                        }
                    })
                    .or_insert(symbol);
            }
            (None, Some("import")) => {
                // Imports use @name when the query isolates the module path,
                // else the whole captured node text (e.g. Rust use items).
                let text = name_text.or(outer_text).unwrap_or("").trim();
                if !text.is_empty() {
                    imports.insert(text.to_string());
                }
            }
            (None, Some("call")) => {
                if let Some(name) = name_text {
                    references.insert(name.to_string());
                }
            }
            _ => {}
        }
    }

    Some(ParsedFile {
        symbols: symbols.into_values().collect(),
        imports: imports.into_iter().collect(),
        references: references.into_iter().collect(),
        lines: lines.len(),
    })
}

/// Inclusive row range (0-based) of a declaration we want to suppress.
type RowRange = std::ops::RangeInclusive<usize>;

/// Row ranges of Rust test modules: any `mod_item` named `tests` OR preceded
/// by a `#[cfg(test)]` attribute. Every symbol whose declaration row falls in
/// one of these ranges is dropped (the test fns, helpers, and the module
/// symbol itself). Returns empty for any non-Rust language — Python/TS test
/// files are separate files that PageRank already sinks, so detecting test
/// constructs inside them is out of scope here.
fn test_module_ranges(lang: Language, tree: &tree_sitter::Tree, source: &[u8]) -> Vec<RowRange> {
    let mut ranges: Vec<RowRange> = Vec::new();
    if lang != Language::Rust {
        return ranges;
    }
    // `mod_item`s can nest, but a test module at any depth taints everything
    // inside; walk the whole tree and collect each qualifying module's span.
    collect_test_modules(tree.root_node(), source, &mut ranges);
    ranges
}

/// Recurse the named tree, pushing the inclusive row span of every `mod_item`
/// that qualifies as a test module.
fn collect_test_modules(node: tree_sitter::Node, source: &[u8], ranges: &mut Vec<RowRange>) {
    let mut cursor = node.walk();
    let children: Vec<tree_sitter::Node> = node.named_children(&mut cursor).collect();
    for child in children {
        if child.kind() == "mod_item" && is_test_module(child, source) {
            ranges.push(child.start_position().row..=child.end_position().row);
            // No need to descend: the whole span is already suppressed.
            continue;
        }
        collect_test_modules(child, source, ranges);
    }
}

/// A `mod_item` is a test module when it is named `tests` or is preceded by a
/// `#[cfg(test)]` attribute. Walks back over intervening attribute/comment
/// siblings so `#[cfg(test)]\n#[allow(..)]\nmod foo` is still detected.
fn is_test_module(module: tree_sitter::Node, source: &[u8]) -> bool {
    if let Some(name) = module.child_by_field_name("name") {
        if name.utf8_text(source) == Ok("tests") {
            return true;
        }
    }
    let mut prev = module.prev_sibling();
    while let Some(node) = prev {
        match node.kind() {
            "attribute_item" => {
                let text = node.utf8_text(source).unwrap_or("");
                // Normalize whitespace so `cfg ( test )` and `cfg(test)` match.
                let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                if compact.contains("cfg(test)") {
                    return true;
                }
            }
            "line_comment" | "block_comment" => {}
            // Any other preceding node ends the attribute run.
            _ => break,
        }
        prev = node.prev_sibling();
    }
    false
}

/// Decide a declaration's visibility from its source signature, per language.
/// Visibility drives the budget ladder's first rung (drop private) and the
/// `--no-private` flag, so it must reflect each language's real rule, not just
/// Python's leading-underscore convention.
///
/// - Python: leading `_` is the private convention.
/// - Rust: an item is public iff its declaration starts with `pub`
///   (`pub`, `pub(crate)`, …). Trait method signatures carry no `pub` keyword
///   (they're as visible as the trait), so they read as private here — a known
///   over-restriction, acceptable since it only bites under a tight budget.
/// - TypeScript: a member is private iff a `private`/`protected` modifier
///   precedes its name; everything else (exports, public members, plain
///   top-level declarations) reads as public.
fn visibility_of(lang: Language, signature: &str, name: &str) -> Visibility {
    match lang {
        Language::Python => {
            if name.starts_with('_') {
                Visibility::Private
            } else {
                Visibility::Public
            }
        }
        Language::Rust => {
            if signature.trim_start().starts_with("pub") {
                Visibility::Public
            } else {
                Visibility::Private
            }
        }
        Language::TypeScript => {
            let before_name = signature.split(name).next().unwrap_or("");
            if before_name
                .split_whitespace()
                .any(|w| w == "private" || w == "protected")
            {
                Visibility::Private
            } else {
                Visibility::Public
            }
        }
    }
}

/// UPPER_SNAKE check for the module-level-assignment constant rule.
fn is_const_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Compile each language's tags.scm once per process. A query that fails to
/// compile is a atlas bug, not a user error: warn once and skip the
/// language rather than crash (FR-12).
fn compiled_query(lang: Language) -> Option<&'static Query> {
    static PYTHON: OnceLock<Option<Query>> = OnceLock::new();
    static TYPESCRIPT: OnceLock<Option<Query>> = OnceLock::new();
    static RUST: OnceLock<Option<Query>> = OnceLock::new();
    match lang {
        Language::Python => PYTHON.get_or_init(|| build_query(lang)).as_ref(),
        Language::TypeScript => TYPESCRIPT.get_or_init(|| build_query(lang)).as_ref(),
        Language::Rust => RUST.get_or_init(|| build_query(lang)).as_ref(),
    }
}

fn build_query(lang: Language) -> Option<Query> {
    let grammar = lang.grammar()?;
    match Query::new(&grammar, lang.tags_query()) {
        Ok(query) => Some(query),
        Err(err) => {
            eprintln!(
                "atlas: internal error: queries/{}/tags.scm failed to compile: {err}",
                lang.name()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_module_ranges, Language};

    #[test]
    fn const_name_convention() {
        assert!(super::is_const_name("API_VERSION"));
        assert!(super::is_const_name("X2"));
        assert!(!super::is_const_name("ApiVersion"));
        assert!(!super::is_const_name("api_version"));
        assert!(!super::is_const_name("_private"));
        assert!(!super::is_const_name(""));
    }

    #[test]
    fn visibility_is_language_aware() {
        use super::{visibility_of, Visibility};
        // Rust: `pub`-prefixed is public, bare is private.
        assert_eq!(
            visibility_of(Language::Rust, "pub fn f()", "f"),
            Visibility::Public
        );
        assert_eq!(
            visibility_of(Language::Rust, "pub(crate) fn f()", "f"),
            Visibility::Public
        );
        assert_eq!(
            visibility_of(Language::Rust, "fn f()", "f"),
            Visibility::Private
        );
        // Python: leading underscore.
        assert_eq!(
            visibility_of(Language::Python, "def _f()", "_f"),
            Visibility::Private
        );
        assert_eq!(
            visibility_of(Language::Python, "def f()", "f"),
            Visibility::Public
        );
        // TypeScript: private/protected member modifiers.
        assert_eq!(
            visibility_of(
                Language::TypeScript,
                "private helperMethod(): void",
                "helperMethod"
            ),
            Visibility::Private
        );
        assert_eq!(
            visibility_of(Language::TypeScript, "protected x(): void", "x"),
            Visibility::Private
        );
        assert_eq!(
            visibility_of(Language::TypeScript, "export function foo()", "foo"),
            Visibility::Public
        );
        assert_eq!(
            visibility_of(Language::TypeScript, "run(): void", "run"),
            Visibility::Public
        );
    }

    fn ranges(source: &str) -> Vec<(usize, usize)> {
        let grammar = Language::Rust.grammar().expect("rust grammar wired");
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).expect("set rust language");
        let tree = parser.parse(source, None).expect("parse");
        test_module_ranges(Language::Rust, &tree, source.as_bytes())
            .into_iter()
            .map(|r| (*r.start(), *r.end()))
            .collect()
    }

    #[test]
    fn detects_tests_named_module() {
        // `mod tests` qualifies on name alone, no attribute needed.
        let r = ranges("mod tests {\n    fn h() {}\n}\n");
        assert_eq!(r, vec![(0, 2)]);
    }

    #[test]
    fn detects_cfg_test_on_differently_named_module() {
        // A non-`tests` name still qualifies via the preceding attribute.
        let r = ranges("#[cfg(test)]\nmod unit_checks {\n    fn h() {}\n}\n");
        assert_eq!(r, vec![(1, 3)]);
    }

    #[test]
    fn cfg_test_through_intervening_attribute() {
        let src = "#[cfg(test)]\n#[allow(dead_code)]\nmod m {\n    fn h() {}\n}\n";
        assert_eq!(ranges(src), vec![(2, 4)]);
    }

    #[test]
    fn ordinary_module_is_not_a_test_module() {
        assert!(ranges("pub mod nested {\n    pub type A = u64;\n}\n").is_empty());
    }

    #[test]
    fn cfg_test_on_non_module_is_ignored() {
        // The task scopes suppression to test *modules*; a `#[cfg(test)]` on a
        // bare fn produces no range (documented edge case).
        assert!(ranges("#[cfg(test)]\nfn lonely() {}\n").is_empty());
    }

    #[test]
    fn non_rust_is_a_noop() {
        // Python source parsed as Rust would be malformed, but the language
        // guard short-circuits before parsing matters.
        let src = "mod tests { fn h() {} }";
        let grammar = Language::Rust.grammar().expect("rust grammar");
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).expect("set language");
        let tree = parser.parse(src, None).expect("parse");
        assert!(test_module_ranges(Language::Python, &tree, src.as_bytes()).is_empty());
        assert!(test_module_ranges(Language::TypeScript, &tree, src.as_bytes()).is_empty());
    }

    // Full extraction (including end-to-end test-module exclusion) is covered
    // by the snapshot test in tests/query_snapshots.rs against the fixtures.
}
