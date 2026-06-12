//! Stage 2 — parse: per-file tree-sitter parse extracting declarations via
//! the embedded `queries/<lang>/tags.scm` query files.
//!
//! Serial in M0; rayon parallelism lands in M1. Unparseable files are
//! skipped and counted, never a panic (FR-12). Files whose language grammar
//! isn't wired yet (TS/JS, Rust until M1) are counted separately.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::discover::SourceFile;
use crate::lang::Language;

/// A declaration extracted from one file.
#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Enum,
    TypeAlias,
    Constant,
    Module,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

/// Parse result for one file: declarations plus outgoing edges-to-be.
#[derive(Clone, Debug, Default)]
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
/// not-yet-wired (FR-12: nothing here ever panics on input).
pub fn parse_all(files: Vec<SourceFile>) -> ParseOutcome {
    let mut out = ParseOutcome {
        files: Vec::new(),
        stats: ParseStats::default(),
    };
    for file in files {
        if file.lang.grammar().is_none() {
            out.stats.unwired_files += 1;
            continue;
        }
        match parse_file(&file) {
            Some(parsed) => {
                out.stats.parsed_files += 1;
                out.stats.total_lines += parsed.lines;
                out.files.push((file, parsed));
            }
            None => out.stats.skipped_files += 1,
        }
    }
    out
}

/// Parse one file with the tree-sitter grammar for its language.
/// Returns `None` for unreadable/unparseable files; the caller counts them
/// and the renderer reports the count in a footer line (FR-12).
pub fn parse_file(file: &SourceFile) -> Option<ParsedFile> {
    let source = std::fs::read_to_string(&file.path).ok()?;
    let grammar = file.lang.grammar()?;
    let query = compiled_query(file.lang)?;

    let mut parser = Parser::new();
    parser.set_language(&grammar).ok()?;
    let tree = parser.parse(&source, None)?;

    let lines: Vec<&str> = source.lines().collect();
    let capture_names = query.capture_names();

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

        for capture in m.captures {
            let capture_name = capture_names[capture.index as usize];
            let text = source.get(capture.node.byte_range()).unwrap_or("");
            if capture_name == "name" {
                name_text = Some(text);
                name_row = capture.node.start_position().row;
            } else if let Some(kind) = capture_name.strip_prefix("definition.") {
                definition = SymbolKind::from_capture(kind);
                outer_text = Some(text);
            } else if let Some(kind) = capture_name.strip_prefix("reference.") {
                reference = Some(kind);
                outer_text = Some(text);
            }
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
                let symbol = Symbol {
                    name: name.to_string(),
                    kind,
                    signature,
                    line: name_row + 1,
                    visibility: if name.starts_with('_') {
                        Visibility::Private
                    } else {
                        Visibility::Public
                    },
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

/// UPPER_SNAKE check for the module-level-assignment constant rule.
fn is_const_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Compile each language's tags.scm once per process. A query that fails to
/// compile is a repomap bug, not a user error: warn once and skip the
/// language rather than crash (FR-12).
fn compiled_query(lang: Language) -> Option<&'static Query> {
    static PYTHON: OnceLock<Option<Query>> = OnceLock::new();
    match lang {
        Language::Python => PYTHON.get_or_init(|| build_query(lang)).as_ref(),
        // TS/JS and Rust grammars land in M1.
        Language::TypeScript | Language::Rust => None,
    }
}

fn build_query(lang: Language) -> Option<Query> {
    let grammar = lang.grammar()?;
    match Query::new(&grammar, lang.tags_query()) {
        Ok(query) => Some(query),
        Err(err) => {
            eprintln!(
                "repomap: internal error: queries/{}/tags.scm failed to compile: {err}",
                lang.name()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_const_name;

    #[test]
    fn const_name_convention() {
        assert!(is_const_name("API_VERSION"));
        assert!(is_const_name("X2"));
        assert!(!is_const_name("ApiVersion"));
        assert!(!is_const_name("api_version"));
        assert!(!is_const_name("_private"));
        assert!(!is_const_name(""));
    }

    // Full extraction is covered by the snapshot test in
    // tests/query_snapshots.rs against tests/queries/fixtures/python.py.
}
