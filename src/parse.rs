//! Stage 2 — parse: per-file tree-sitter parse extracting declarations via
//! the embedded `queries/<lang>/tags.scm` query files.
//!
//! Unparseable files are skipped and counted, never a panic (FR-12). Files
//! whose language grammar isn't wired yet are counted separately. The expensive
//! tree-sitter parse of cache-miss files runs in parallel (rayon), while cache
//! access and output assembly stay sequential to keep output deterministic
//! (NFR-4).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use rayon::prelude::*;
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
    // Phase 1 (sequential): read each file and classify it. Cache lookups need
    // `&mut Cache`, and the read is cheap next to the parse, so this stays
    // sequential.
    let mut slots: Vec<(SourceFile, Slot)> = Vec::with_capacity(files.len());
    for file in files {
        let slot = if file.lang.grammar().is_none() {
            Slot::Unwired
        } else if let Ok(source) = std::fs::read_to_string(&file.path) {
            let hash = crate::cache::content_hash(&source);
            match cache.get(&file.rel, hash) {
                Some(parsed) => Slot::Ready(parsed),
                None => Slot::ToParse { source, hash },
            }
        } else {
            Slot::Skipped
        };
        slots.push((file, slot));
    }

    // Phase 2 (parallel): run the expensive tree-sitter parse on the cache
    // misses. `parse_source` is pure, so this is embarrassingly parallel; results
    // are keyed by slot index so assembly stays ordered.
    let mut parsed_misses: Vec<Option<ParsedFile>> = (0..slots.len()).map(|_| None).collect();
    let results: Vec<(usize, ParsedFile)> = slots
        .par_iter()
        .enumerate()
        .filter_map(|(i, (file, slot))| match slot {
            Slot::ToParse { source, .. } => parse_source(file, source).map(|p| (i, p)),
            _ => None,
        })
        .collect();
    for (i, parsed) in results {
        parsed_misses[i] = Some(parsed);
    }

    // Phase 3 (sequential): assemble in input order, updating the cache and
    // stats. A miss with no parse result failed to parse → skipped (FR-12).
    let mut out = ParseOutcome {
        files: Vec::with_capacity(slots.len()),
        stats: ParseStats::default(),
    };
    for (idx, (file, slot)) in slots.into_iter().enumerate() {
        let parsed = match slot {
            Slot::Unwired => {
                out.stats.unwired_files += 1;
                continue;
            }
            Slot::Skipped => {
                out.stats.skipped_files += 1;
                continue;
            }
            Slot::Ready(parsed) => parsed,
            Slot::ToParse { hash, .. } => match parsed_misses[idx].take() {
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

/// Per-file classification from Phase 1 of [`parse_all_cached`].
enum Slot {
    /// Language grammar not wired (counted, not parsed).
    Unwired,
    /// Unreadable / non-UTF-8 file (counted, not parsed).
    Skipped,
    /// Cache hit — reuse the stored parse.
    Ready(ParsedFile),
    /// Cache miss — parse `source` in parallel, then cache under `hash`.
    ToParse { source: String, hash: u64 },
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

    // Rows of C++ class/struct members in a `private:`/`protected:` section.
    // The access specifier is a sibling node, never on the member's own line,
    // so visibility for those members is decided here, not from the signature.
    // Empty for every non-C++ language.
    let cpp_private_rows = cpp_private_member_rows(file.lang, &tree, source.as_bytes());

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
                // Python/TS capture constants from broad assignment patterns, so
                // keep only conventional UPPER_SNAKE names there. Go and Rust
                // capture an explicit `const`/`static` construct — those are
                // constants regardless of case (Go's exported consts are
                // PascalCase), so they are not name-filtered.
                if kind == SymbolKind::Constant
                    && matches!(file.lang, Language::Python | Language::TypeScript)
                    && !is_const_name(name)
                {
                    continue;
                }
                let signature = lines
                    .get(name_row)
                    .map(|l| l.trim().to_string())
                    .unwrap_or_default();
                let mut visibility = visibility_of(file.lang, &signature, name);
                // C++ members in a private/protected access section are internal
                // API regardless of their signature text (see cpp_private_rows).
                if cpp_private_rows.contains(&name_row) {
                    visibility = Visibility::Private;
                }
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

/// Rows (0-based) of C++ class/struct members that fall in a `private:` or
/// `protected:` access-specifier section. A member declared on one of these
/// rows is internal API and must read as [`Visibility::Private`] regardless of
/// its signature text — the access specifier is a sibling node, never on the
/// member's own line, so [`visibility_of`] can't see it without this pass.
///
/// Access rules encoded here:
/// - `class` defaults to private: members before the first `access_specifier`
///   are private.
/// - `struct` defaults to public: members before the first specifier are public.
/// - After a specifier, the section runs until the next specifier; `private`
///   and `protected` sections are collected, `public` sections are not.
///
/// Returns empty for any language other than C++ (C has no access specifiers).
fn cpp_private_member_rows(
    lang: Language,
    tree: &tree_sitter::Tree,
    source: &[u8],
) -> BTreeSet<usize> {
    let mut rows: BTreeSet<usize> = BTreeSet::new();
    if lang != Language::Cpp {
        return rows;
    }
    collect_cpp_private_rows(tree.root_node(), source, &mut rows);
    rows
}

/// Recurse the named tree; for every class/struct body, walk its members in
/// source order tracking the current access section and record the rows of
/// members in a private/protected section.
fn collect_cpp_private_rows(node: tree_sitter::Node, source: &[u8], rows: &mut BTreeSet<usize>) {
    let kind = node.kind();
    if kind == "class_specifier" || kind == "struct_specifier" {
        if let Some(body) = node.child_by_field_name("body") {
            // `class` defaults to private, `struct` defaults to public.
            let mut private_section = kind == "class_specifier";
            let mut cursor = body.walk();
            for child in body.named_children(&mut cursor) {
                if child.kind() == "access_specifier" {
                    // Specifier text is the bare keyword (e.g. "private").
                    let keyword = child.utf8_text(source).unwrap_or("");
                    private_section = matches!(keyword, "private" | "protected");
                    continue;
                }
                if private_section {
                    // A member declaration can span lines; mark every row it
                    // covers so the member's name row (wherever it lands) is hit.
                    for row in child.start_position().row..=child.end_position().row {
                        rows.insert(row);
                    }
                }
            }
        }
    }
    // Descend regardless: classes nest, and a class may sit inside a namespace,
    // function, or another class body.
    let mut cursor = node.walk();
    let children: Vec<tree_sitter::Node> = node.named_children(&mut cursor).collect();
    for child in children {
        collect_cpp_private_rows(child, source, rows);
    }
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
        Language::TypeScript | Language::Java => {
            // TS: `private`/`protected` member modifiers mark non-public. Java
            // shares the same rule — a `private`/`protected` modifier before the
            // name is non-public; `public`, package-private, and top-level
            // declarations read as public (the visible API surface).
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
        Language::Go => {
            // Go: an identifier is exported iff its first letter is uppercase.
            // No keyword involved — the name alone decides.
            if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                Visibility::Public
            } else {
                Visibility::Private
            }
        }
        Language::C | Language::Cpp => {
            // C/C++: a `static` free function or file-scope variable has
            // internal linkage (file-private); everything else is the externally
            // visible surface. C++ class-member access specifiers live on a
            // sibling node, not the member's signature line, so they're handled
            // by [`cpp_private_member_rows`] in `parse_source`, which overrides
            // this default to Private for members in a private/protected section.
            let before_name = signature.split(name).next().unwrap_or("");
            if before_name.split_whitespace().any(|w| w == "static") {
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
    static GO: OnceLock<Option<Query>> = OnceLock::new();
    static JAVA: OnceLock<Option<Query>> = OnceLock::new();
    static C: OnceLock<Option<Query>> = OnceLock::new();
    static CPP: OnceLock<Option<Query>> = OnceLock::new();
    match lang {
        Language::Python => PYTHON.get_or_init(|| build_query(lang)).as_ref(),
        Language::TypeScript => TYPESCRIPT.get_or_init(|| build_query(lang)).as_ref(),
        Language::Rust => RUST.get_or_init(|| build_query(lang)).as_ref(),
        Language::Go => GO.get_or_init(|| build_query(lang)).as_ref(),
        Language::Java => JAVA.get_or_init(|| build_query(lang)).as_ref(),
        Language::C => C.get_or_init(|| build_query(lang)).as_ref(),
        Language::Cpp => CPP.get_or_init(|| build_query(lang)).as_ref(),
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

    /// Visibility of a named C++ symbol after full extraction. Returns `None`
    /// if no symbol with that name was extracted.
    fn cpp_visibility(source: &str, name: &str) -> Option<super::Visibility> {
        let file = crate::discover::SourceFile {
            path: std::path::PathBuf::from("scratch.cpp"),
            rel: "scratch.cpp".to_string(),
            lang: Language::Cpp,
        };
        let parsed = super::parse_source(&file, source)?;
        parsed
            .symbols
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.visibility)
    }

    #[test]
    fn cpp_class_defaults_members_to_private() {
        use super::Visibility;
        // `class` defaults to private: members before any specifier are private,
        // members after `public:` are public.
        let src = "class Widget {\n    int hidden_;\npublic:\n    int api();\n};\n";
        assert_eq!(cpp_visibility(src, "hidden_"), Some(Visibility::Private));
        assert_eq!(cpp_visibility(src, "api"), Some(Visibility::Public));
    }

    #[test]
    fn cpp_struct_defaults_members_to_public() {
        use super::Visibility;
        // `struct` defaults to public; an explicit `private:` flips the section.
        let src = "struct Bag {\n    int openField;\nprivate:\n    int closedField;\n};\n";
        assert_eq!(cpp_visibility(src, "openField"), Some(Visibility::Public));
        assert_eq!(
            cpp_visibility(src, "closedField"),
            Some(Visibility::Private)
        );
    }

    #[test]
    fn cpp_protected_section_is_private() {
        use super::Visibility;
        let src = "class C {\nprotected:\n    int guarded();\n};\n";
        assert_eq!(cpp_visibility(src, "guarded"), Some(Visibility::Private));
    }

    #[test]
    fn cpp_free_function_linkage_unchanged() {
        use super::Visibility;
        // A `static` free function keeps file-private internal linkage; a plain
        // free function stays public. Access specifiers must not affect these.
        let src =
            "static int internalFn(int x) { return x; }\nint externalFn(int y) { return y; }\n";
        assert_eq!(cpp_visibility(src, "internalFn"), Some(Visibility::Private));
        assert_eq!(cpp_visibility(src, "externalFn"), Some(Visibility::Public));
    }
}
