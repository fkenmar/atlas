//! Stage 3 — link: resolve imports and defs↔refs into a directed graph of
//! file and symbol nodes. Resolution is best-effort and syntactic (module
//! paths, relative imports) — no type checker (PRD §5.1).
//!
//! The graph is index-based per ADR-0002: nodes live in a `Vec`, edges hold
//! `usize` handles. No references, lifetimes, `Rc`, or interior mutability
//! in graph structures.
//!
//! Node layout invariant: the first `files.len()` nodes are the File nodes,
//! one per parsed file, in discover's sorted order — so a File node's index
//! equals its index into the `files` slice. Symbol nodes follow.
//!
//! Two edge kinds. An import edge runs File → File (the importer points at the
//! imported file). A reference edge runs File → Symbol (a call site points at
//! every same-named definition), so widely-called symbols accumulate in-edges
//! and rank high under PageRank. Symbol nodes are sinks here; PageRank
//! (stage 4) handles their dangling rank. Edge lists are sorted and
//! deduplicated for determinism (NFR-4).

use std::collections::HashMap;

use crate::discover::SourceFile;
use crate::lang::Language;
use crate::parse::ParsedFile;

pub struct Graph {
    pub nodes: Vec<Node>,
    /// Outgoing adjacency: `edges[i]` lists the node indices that node `i`
    /// points to (imports / references), sorted and deduplicated.
    pub edges: Vec<Vec<usize>>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    /// File path (File nodes) or qualified symbol name (Symbol nodes).
    pub label: String,
    /// Index into the `files` slice this node belongs to. For a File node,
    /// its own file; for a Symbol node, the file that defines it.
    pub file: usize,
    /// For Symbol nodes, the index into `files[file].1.symbols`; `None` for
    /// File nodes.
    pub symbol: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Symbol,
}

impl Graph {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Build the import/reference graph from all parsed files.
pub fn link(files: &[(SourceFile, ParsedFile)]) -> Graph {
    let num_files = files.len();

    // 1. File nodes first, so a File node's index == its index into `files`.
    let mut nodes: Vec<Node> = Vec::with_capacity(num_files);
    for (src, _) in files {
        nodes.push(Node {
            kind: NodeKind::File,
            label: src.rel.clone(),
            file: nodes.len(),
            symbol: None,
        });
    }

    // 2. Symbol nodes, plus a name → symbol-node-index lookup for references.
    let mut symbols_by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (file_idx, (_, parsed)) in files.iter().enumerate() {
        for (sym_idx, sym) in parsed.symbols.iter().enumerate() {
            let node_idx = nodes.len();
            nodes.push(Node {
                kind: NodeKind::Symbol,
                label: sym.name.clone(),
                file: file_idx,
                symbol: Some(sym_idx),
            });
            symbols_by_name
                .entry(sym.name.clone())
                .or_default()
                .push(node_idx);
        }
    }

    // 3. Path index for best-effort import resolution.
    let index = FileIndex::build(files);

    // 4. Edges.
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (file_idx, (src, parsed)) in files.iter().enumerate() {
        // Import edges: importer file → imported file.
        for import in &parsed.imports {
            if let Some(target) = index.resolve(import, src.lang, &src.rel) {
                if target != file_idx {
                    edges[file_idx].push(target);
                }
            }
        }
        // Reference edges: file → every same-named definition (cross-file is
        // the signal we want; a file referencing its own symbol is skipped so
        // internal calls don't inflate a file's own symbols).
        for reference in &parsed.references {
            if let Some(targets) = symbols_by_name.get(reference) {
                for &sym_node in targets {
                    if nodes[sym_node].file != file_idx {
                        edges[file_idx].push(sym_node);
                    }
                }
            }
        }
    }

    // 5. Determinism: sort + dedup each adjacency list.
    for adj in &mut edges {
        adj.sort_unstable();
        adj.dedup();
    }

    Graph { nodes, edges }
}

/// Lookup tables for resolving an import string to a file index. Built once
/// per `link` call; never iterated for output (NFR-4 — lookups only).
struct FileIndex {
    /// Extension-stripped, `/`-joined path → file index, e.g.
    /// `src/auth/service`. A package `__init__`/`mod`/`index` file also
    /// registers its directory.
    by_path: HashMap<String, usize>,
    /// Basename (no extension) → file indices, for last-segment fallback.
    by_basename: HashMap<String, Vec<usize>>,
}

impl FileIndex {
    fn build(files: &[(SourceFile, ParsedFile)]) -> FileIndex {
        let mut by_path: HashMap<String, usize> = HashMap::new();
        let mut by_basename: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, (src, _)) in files.iter().enumerate() {
            let stem = strip_extension(&src.rel);
            by_path.entry(stem.to_string()).or_insert(idx);
            let base = stem.rsplit('/').next().unwrap_or(stem);
            // A package entry file stands in for its directory.
            if matches!(base, "__init__" | "mod" | "index") {
                if let Some(dir) = stem.rsplit_once('/').map(|(d, _)| d) {
                    by_path.entry(dir.to_string()).or_insert(idx);
                }
            } else {
                by_basename.entry(base.to_string()).or_default().push(idx);
            }
        }
        FileIndex {
            by_path,
            by_basename,
        }
    }

    fn resolve(&self, import: &str, lang: Language, from_rel: &str) -> Option<usize> {
        match lang {
            Language::Python => self.resolve_python(import),
            Language::TypeScript => self.resolve_typescript(import, from_rel),
            Language::Rust => self.resolve_rust(import),
        }
    }

    /// `a.b.c` → `a/b/c`, matched against in-repo files. A name with no path
    /// match is treated as external (stdlib/third-party) and creates no edge:
    /// resolving a bare name by basename would falsely bind, e.g., `import
    /// models` to a nested local `models.py` that merely shares the name.
    /// Leading-dot relative imports (`from . import x`) are not resolved yet
    /// (M1 gap — the query doesn't capture `relative_import` nodes).
    fn resolve_python(&self, import: &str) -> Option<usize> {
        let path = import.trim().replace('.', "/");
        self.by_path.get(&path).copied()
    }

    /// Relative specifiers (`./x`, `../y/z`) resolve against the importer's
    /// directory; bare specifiers (`react`, `node:path`) are external and
    /// miss. The captured `@name` is the quoted module string, so trim quotes.
    fn resolve_typescript(&self, import: &str, from_rel: &str) -> Option<usize> {
        let spec = import.trim().trim_matches(|c| c == '"' || c == '\'');
        if !spec.starts_with('.') {
            return None; // external package
        }
        let from_dir = from_rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let joined = normalize_path(from_dir, spec)?;
        self.by_path
            .get(&joined)
            .copied()
            .or_else(|| self.by_path.get(&format!("{joined}/index")).copied())
    }

    /// `use crate::a::b::C;` is best-effort: strip any visibility modifier and
    /// the `use`/`;` wrapper, then try the longest path match, then a unique
    /// basename match on the top module segment only. No module-tree
    /// resolution (no type checker), so this is deliberately fuzzy.
    fn resolve_rust(&self, import: &str) -> Option<usize> {
        let raw = import.trim().trim_end_matches(';');
        // The query captures the whole `use_declaration`, so a leading
        // `pub`/`pub(crate)` modifier survives; anchor on the `use ` keyword
        // (B2) rather than assuming the string starts with it.
        let body = match raw.find("use ") {
            Some(i) => raw[i + 4..].trim(),
            None => raw.trim(),
        };
        // Leading path only: stop at a `{` group or `*` glob, drop an ` as `
        // alias (S2).
        let path = body.split(['{', '*']).next().unwrap_or(body);
        let path = path.split(" as ").next().unwrap_or(path);
        let segments: Vec<&str> = path
            .split("::")
            .map(str::trim)
            .filter(|s| !s.is_empty() && !matches!(*s, "crate" | "self" | "super"))
            .collect();
        let &first = segments.first()?;
        // Longest-prefix path match: full join first, then shorter prefixes
        // (the trailing segment is usually the imported type/fn).
        for end in (1..=segments.len()).rev() {
            let candidate = segments[..end].join("/");
            if let Some(&idx) = self.by_path.get(&candidate) {
                return Some(idx);
            }
        }
        // Fallback: the top module segment only, by unique basename. Trying
        // every segment would bind `crate::missing::auth::login` to an
        // unrelated `auth.rs` (S2); the top segment is the least-bad guess.
        self.unique_basename(first)
    }

    fn unique_basename(&self, base: &str) -> Option<usize> {
        match self.by_basename.get(base) {
            Some(v) if v.len() == 1 => Some(v[0]),
            _ => None,
        }
    }
}

fn strip_extension(rel: &str) -> &str {
    match rel.rsplit_once('.') {
        // Only strip a trailing path-component extension, not a dotted dir.
        Some((stem, ext)) if !ext.contains('/') => stem,
        _ => rel,
    }
}

/// Resolve a `./`-relative specifier against a base directory, collapsing
/// `.` and `..` segments into a `/`-joined, extension-less path key. Returns
/// `None` when a `..` escapes above the base — the target then lies outside
/// the scanned root, so any `by_path` match would be a false edge (S1).
fn normalize_path(base_dir: &str, spec: &str) -> Option<String> {
    let mut parts: Vec<&str> = if base_dir.is_empty() {
        Vec::new()
    } else {
        base_dir.split('/').collect()
    };
    for seg in spec.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?; // underflow = escaped the root → unresolvable
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{Symbol, SymbolKind, Visibility};
    use std::path::PathBuf;

    fn file(rel: &str, lang: Language) -> SourceFile {
        SourceFile {
            path: PathBuf::from(rel),
            rel: rel.to_string(),
            lang,
        }
    }

    fn sym(name: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: format!("def {name}()"),
            line: 1,
            visibility: Visibility::Public,
        }
    }

    fn parsed(symbols: &[&str], imports: &[&str], references: &[&str]) -> ParsedFile {
        ParsedFile {
            symbols: symbols.iter().map(|n| sym(n)).collect(),
            imports: imports.iter().map(|s| s.to_string()).collect(),
            references: references.iter().map(|s| s.to_string()).collect(),
            lines: 10,
        }
    }

    /// Locate the Symbol node for `name` defined in file index `file`.
    fn symbol_node(g: &Graph, file: usize, name: &str) -> usize {
        g.nodes
            .iter()
            .position(|n| n.kind == NodeKind::Symbol && n.file == file && n.label == name)
            .expect("symbol node exists")
    }

    #[test]
    fn file_nodes_come_first_and_are_index_aligned() {
        let files = vec![
            (file("a.py", Language::Python), parsed(&["f"], &[], &[])),
            (file("b.py", Language::Python), parsed(&["g"], &[], &[])),
        ];
        let g = link(&files);
        assert_eq!(g.nodes[0].kind, NodeKind::File);
        assert_eq!(g.nodes[0].label, "a.py");
        assert_eq!(g.nodes[1].kind, NodeKind::File);
        assert_eq!(g.nodes[1].label, "b.py");
        // Two files + two symbols.
        assert_eq!(g.node_count(), 4);
    }

    #[test]
    fn reference_edge_points_to_cross_file_definition() {
        // b.py calls `helper`, defined in a.py.
        let files = vec![
            (
                file("a.py", Language::Python),
                parsed(&["helper"], &[], &[]),
            ),
            (
                file("b.py", Language::Python),
                parsed(&["main"], &[], &["helper"]),
            ),
        ];
        let g = link(&files);
        let helper = symbol_node(&g, 0, "helper");
        // File b (index 1) → helper symbol node.
        assert!(g.edges[1].contains(&helper));
    }

    #[test]
    fn same_file_reference_makes_no_edge() {
        // a.py defines and calls `helper` itself — no edge (no cross-file signal).
        let files = vec![(
            file("a.py", Language::Python),
            parsed(&["helper", "main"], &[], &["helper"]),
        )];
        let g = link(&files);
        assert!(g.edges[0].is_empty());
    }

    #[test]
    fn unresolvable_reference_creates_no_edge() {
        let files = vec![(
            file("a.py", Language::Python),
            parsed(&["main"], &[], &["nonexistent"]),
        )];
        let g = link(&files);
        assert!(g.edges[0].is_empty());
    }

    #[test]
    fn python_import_resolves_to_in_repo_file() {
        // pkg/app.py imports `pkg.util`; pkg/util.py exists.
        let files = vec![
            (
                file("pkg/app.py", Language::Python),
                parsed(&["run"], &["pkg.util"], &[]),
            ),
            (
                file("pkg/util.py", Language::Python),
                parsed(&["helper"], &[], &[]),
            ),
        ];
        let g = link(&files);
        // app (0) → util (1).
        assert!(g.edges[0].contains(&1));
    }

    #[test]
    fn python_package_init_resolves_by_directory() {
        let files = vec![
            (
                file("app.py", Language::Python),
                parsed(&["run"], &["pkg"], &[]),
            ),
            (
                file("pkg/__init__.py", Language::Python),
                parsed(&["setup"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(g.edges[0].contains(&1)); // app → pkg/__init__
    }

    #[test]
    fn external_python_import_skipped() {
        let files = vec![(
            file("a.py", Language::Python),
            parsed(&["run"], &["os", "typing"], &[]),
        )];
        let g = link(&files);
        assert!(g.edges[0].is_empty());
    }

    #[test]
    fn typescript_relative_import_resolves() {
        // src/app.ts imports "./util"; src/util.ts exists.
        let files = vec![
            (
                file("src/app.ts", Language::TypeScript),
                parsed(&["run"], &["\"./util\""], &[]),
            ),
            (
                file("src/util.ts", Language::TypeScript),
                parsed(&["helper"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(g.edges[0].contains(&1));
    }

    #[test]
    fn typescript_parent_relative_and_index_resolve() {
        // src/api/routes.ts imports "../util" → src/util/index.ts.
        let files = vec![
            (
                file("src/api/routes.ts", Language::TypeScript),
                parsed(&["reg"], &["\"../util\""], &[]),
            ),
            (
                file("src/util/index.ts", Language::TypeScript),
                parsed(&["helper"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(g.edges[0].contains(&1));
    }

    #[test]
    fn typescript_external_import_skipped() {
        let files = vec![(
            file("src/app.ts", Language::TypeScript),
            parsed(&["run"], &["\"node:path\"", "\"react\""], &[]),
        )];
        let g = link(&files);
        assert!(g.edges[0].is_empty());
    }

    #[test]
    fn rust_use_resolves_to_module_file() {
        // main.rs `use crate::link::Graph;` → src/link.rs (basename "link").
        let files = vec![
            (
                file("src/main.rs", Language::Rust),
                parsed(&["main"], &["use crate::link::Graph;"], &[]),
            ),
            (
                file("src/link.rs", Language::Rust),
                parsed(&["link"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(g.edges[0].contains(&1));
    }

    #[test]
    fn edges_are_sorted_and_deduplicated() {
        // Two references to the same symbol must collapse to one edge.
        let files = vec![
            (file("a.py", Language::Python), parsed(&["t"], &[], &[])),
            (
                file("b.py", Language::Python),
                parsed(&["main"], &[], &["t", "t"]),
            ),
        ];
        let g = link(&files);
        let t = symbol_node(&g, 0, "t");
        assert_eq!(g.edges[1].iter().filter(|&&n| n == t).count(), 1);
        // Sorted invariant.
        let mut sorted = g.edges[1].clone();
        sorted.sort_unstable();
        assert_eq!(g.edges[1], sorted);
    }

    // ---- regression guards from the link-stage review ----

    #[test]
    fn python_bare_import_does_not_falsely_bind_nested_file() {
        // B1: `import service` must NOT resolve to src/auth/service.py — that
        // file is not importable as a bare top-level name, so binding it would
        // be a false File→File edge that inflates its PageRank.
        let files = vec![
            (
                file("app.py", Language::Python),
                parsed(&["main"], &["service"], &[]),
            ),
            (
                file("src/auth/service.py", Language::Python),
                parsed(&["svc"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(g.edges[0].is_empty());
    }

    #[test]
    fn rust_pub_use_resolves_like_plain_use() {
        // B2: the query captures the whole use_declaration, so `pub`/
        // `pub(crate)` prefix the import text; resolution must still find the
        // module file.
        for stmt in [
            "pub use crate::link::Graph;",
            "pub(crate) use crate::link::Graph;",
        ] {
            let files = vec![
                (
                    file("src/lib.rs", Language::Rust),
                    parsed(&["x"], &[stmt], &[]),
                ),
                (
                    file("src/link.rs", Language::Rust),
                    parsed(&["link"], &[], &[]),
                ),
            ];
            let g = link(&files);
            assert!(
                g.edges[0].contains(&1),
                "{stmt} should resolve to src/link.rs"
            );
        }
    }

    #[test]
    fn typescript_import_escaping_root_makes_no_edge() {
        // S1: src/util.ts importing "../../shared" escapes the scanned root;
        // even if a top-level shared.ts exists it must not bind.
        let files = vec![
            (
                file("src/util.ts", Language::TypeScript),
                parsed(&["u"], &["\"../../shared\""], &[]),
            ),
            (
                file("shared.ts", Language::TypeScript),
                parsed(&["s"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(!g.edges[0].contains(&1));
    }

    #[test]
    fn rust_ambiguous_basename_makes_no_edge() {
        // Two files named util.rs: `use crate::util::helper;` has no unique
        // basename match, so no import edge (the len()==1 guard).
        let files = vec![
            (
                file("src/main.rs", Language::Rust),
                parsed(&["m"], &["use crate::util::helper;"], &[]),
            ),
            (
                file("src/a/util.rs", Language::Rust),
                parsed(&["h1"], &[], &[]),
            ),
            (
                file("src/b/util.rs", Language::Rust),
                parsed(&["h2"], &[], &[]),
            ),
        ];
        let g = link(&files);
        assert!(g.edges[0].is_empty());
    }

    #[test]
    fn reference_fans_out_to_all_same_named_definitions() {
        // The cross-file fan-out is the core ranking signal: a call to `run`
        // edges to every file's `run` definition.
        let files = vec![
            (file("a.py", Language::Python), parsed(&["run"], &[], &[])),
            (file("b.py", Language::Python), parsed(&["run"], &[], &[])),
            (
                file("c.py", Language::Python),
                parsed(&["main"], &[], &["run"]),
            ),
        ];
        let g = link(&files);
        let run_a = symbol_node(&g, 0, "run");
        let run_b = symbol_node(&g, 1, "run");
        assert!(g.edges[2].contains(&run_a));
        assert!(g.edges[2].contains(&run_b));
    }
}
