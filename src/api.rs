//! Supported library API for embedding atlas (#69).
//!
//! Agent frameworks can produce a map programmatically instead of shelling out:
//!
//! ```no_run
//! use std::path::Path;
//! use atlas::api::{build_map, MapOptions};
//!
//! let map = build_map(Path::new("."), &MapOptions::default())?;
//! let markdown = atlas::render::markdown::render(&map);
//! println!("{markdown}");
//! # Ok::<(), atlas::api::MapError>(())
//! ```
//!
//! [`build_map`] runs the full pipeline (discover → parse → link → rank →
//! budget) and returns a [`BudgetedMap`]; render it with any
//! [`crate::render`] renderer. **Stability:** this module is the *supported*
//! embedding surface. The other `pub` modules (`parse`, `link`, `rank`,
//! `budget`, …) are exposed for the binary and tests and may change between
//! minor releases without notice; depend on `atlas::api` and the renderers.
//! Library-API breaks bump the crate minor version pre-1.0 — tracked separately
//! from the CLI flags and the JSON/XML output schemas.

use std::path::Path;

use crate::budget::{pack, BudgetOptions, BudgetedMap, TiktokenCounter, DEFAULT_BUDGET};
use crate::lang::Language;

/// Options for [`build_map`]. Use `MapOptions::default()` for the CLI defaults
/// (2,048-token budget, all languages, cache on).
#[derive(Debug, Clone)]
pub struct MapOptions {
    /// Target token budget for the map.
    pub budget: usize,
    /// Drop private symbols (public API surface only).
    pub no_private: bool,
    /// Restrict to these languages; empty means every supported language.
    pub langs: Vec<Language>,
    /// Boost these paths (files or directory prefixes, relative to the root) in
    /// the ranking — the personalization seeds for PageRank, the library
    /// equivalent of the CLI `--focus`. Empty means an unfocused (uniform) rank.
    pub focus: Vec<String>,
    /// Read/write the on-disk parse cache under `<root>/.atlas/cache`. Set
    /// `false` for a read-only run that never writes to the target tree.
    pub cache: bool,
}

impl Default for MapOptions {
    fn default() -> Self {
        MapOptions {
            budget: DEFAULT_BUDGET,
            no_private: false,
            langs: Vec::new(),
            focus: Vec::new(),
            cache: true,
        }
    }
}

/// Why [`build_map`] could not produce a map.
#[derive(Debug)]
pub enum MapError {
    /// No supported source files were found under the root.
    NoSupportedFiles,
    /// `langs` was set but matched none of the discovered files.
    NoMatchingLanguage,
    /// The tiktoken tokenizer could not be initialized.
    Tokenizer(String),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::NoSupportedFiles => write!(f, "no supported source files found"),
            MapError::NoMatchingLanguage => {
                write!(
                    f,
                    "the language filter matched none of the discovered files"
                )
            }
            MapError::Tokenizer(e) => write!(f, "could not initialize the tokenizer: {e}"),
        }
    }
}

impl std::error::Error for MapError {}

/// Build a budgeted structural map of the tree at `root`. The supported entry
/// point for embedding atlas (#69); the CLI and MCP server are thin wrappers
/// over the same pipeline. `--focus` ranking is CLI-only for now.
pub fn build_map(root: &Path, opts: &MapOptions) -> Result<BudgetedMap, MapError> {
    let (repo_name, outcome) = discover_and_parse(root, opts)?;
    let graph = crate::link::link(&outcome.files);
    let seeds = resolve_focus_seeds(&outcome.files, &opts.focus);
    let ranking = crate::rank::rank(&graph, &seeds);
    let counter = TiktokenCounter::cl100k().map_err(MapError::Tokenizer)?;
    let budget_opts = BudgetOptions {
        budget_tokens: opts.budget,
        no_private: opts.no_private,
    };
    Ok(pack(
        &outcome.files,
        &graph,
        &ranking,
        &repo_name,
        outcome.stats,
        &budget_opts,
        &counter,
    ))
}

/// A single located declaration returned by [`find_symbol`] (#26) — enough for
/// an agent to jump straight to the definition without reading the whole map.
#[derive(Debug, Clone)]
pub struct SymbolHit {
    /// The declared name (exact match of the query).
    pub name: String,
    /// Symbol kind: `function`, `method`, `class`, `interface`, `enum`,
    /// `type`, `constant`, `module`, or `field`.
    pub kind: &'static str,
    /// First-line signature (no body).
    pub signature: String,
    /// Repo-relative path of the defining file.
    pub file: String,
    /// 1-based line of the declaration name.
    pub line: usize,
    /// `public` or `private` (per-language visibility rules).
    pub visibility: &'static str,
}

/// A symbol resolved from an anchor by [`expand_symbol`], plus its defining
/// file's one-hop file-level neighbors — the just-in-time detail for
/// progressive disclosure (ADR 0009). Neighbors are file-granular (the imports
/// and importers already in the map), never a symbol-level call graph: resolving
/// who *calls* a symbol is code intelligence (an LSP's job), a hard non-goal.
#[derive(Debug, Clone)]
pub struct SymbolExpansion {
    /// The located declaration (signature, kind, file, line, visibility).
    pub hit: SymbolHit,
    /// Repo-relative paths the defining file imports, sorted and deduplicated.
    pub imports: Vec<String>,
    /// Repo-relative paths of files that import the defining file, sorted.
    pub used_by: Vec<String>,
}

/// A compact stable anchor entry for progressive disclosure (ADR 0009). It is
/// intentionally metadata-only: no signatures, no bodies, and no semantic call
/// graph. Use [`expand_symbol`] to expand one anchor when detail is needed.
#[derive(Debug, Clone)]
pub struct SymbolIndexEntry {
    /// Stable anchor: `relpath#name`, with `@line` only for same-file duplicate
    /// names.
    pub anchor: String,
    /// The declared name.
    pub name: String,
    /// Symbol kind: `function`, `class`, `type`, etc.
    pub kind: &'static str,
    /// Repo-relative path of the defining file.
    pub file: String,
    /// 1-based line of the declaration name.
    pub line: usize,
    /// `public` or `private` (per-language visibility rules).
    pub visibility: &'static str,
}

/// A budgeted thin symbol index for progressive disclosure. Entries are
/// type-like declarations first, then functions/constants, each tier ordered by
/// file rank and source order. The index is designed to be cheap to keep in
/// context and paired with [`expand_symbol`] for just-in-time detail.
#[derive(Debug, Clone)]
pub struct SymbolIndexMap {
    pub repo_name: String,
    pub target_tokens: usize,
    pub rendered_tokens: usize,
    pub total_loc: usize,
    pub total_files: usize,
    pub entries: Vec<SymbolIndexEntry>,
    pub skipped_files: usize,
    pub unwired_files: usize,
}

/// Find every declaration named `name` (exact match) across the tree at `root`,
/// in deterministic (file, line) order. The library equivalent of "where is X
/// defined?" — returns each site so an agent can navigate multi-definition
/// names. Honors `opts.langs` (language filter), `opts.no_private` (drops
/// private hits), and `opts.cache`; `opts.budget`/`opts.focus` are ignored
/// (no ranking or packing happens). An empty result is `Ok(vec![])`, not an
/// error.
pub fn find_symbol(root: &Path, name: &str, opts: &MapOptions) -> Result<Vec<SymbolHit>, MapError> {
    use crate::parse::Visibility;
    let (_repo_name, outcome) = discover_and_parse(root, opts)?;
    let mut hits = Vec::new();
    for (src, parsed) in &outcome.files {
        for sym in &parsed.symbols {
            if sym.name != name {
                continue;
            }
            if opts.no_private && sym.visibility != Visibility::Public {
                continue;
            }
            hits.push(SymbolHit {
                name: sym.name.clone(),
                kind: crate::render::json::kind_name(sym.kind),
                signature: sym.signature.clone(),
                file: src.rel.clone(),
                line: sym.line,
                visibility: crate::render::json::visibility_name(sym.visibility),
            });
        }
    }
    // Deterministic (NFR-4): files arrive in discover's sorted order and a
    // file's symbols are line-sorted, so (file, line) is already stable; sort
    // explicitly to be robust to future ordering changes.
    hits.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    Ok(hits)
}

/// Build the opt-in thin anchor index described in ADR 0009. It honors
/// `opts.langs`, `opts.no_private`, `opts.cache`, and `opts.budget`; `focus` is
/// intentionally ignored so anchors remain a repo-wide discovery surface. The
/// output has no signatures: callers expand selected anchors through
/// [`expand_symbol`].
pub fn build_symbol_index(root: &Path, opts: &MapOptions) -> Result<SymbolIndexMap, MapError> {
    use crate::parse::{SymbolKind, Visibility};
    let (repo_name, outcome) = discover_and_parse(root, opts)?;
    let graph = crate::link::link(&outcome.files);
    let ranking = crate::rank::rank(&graph, &[]);

    let mut order: Vec<usize> = (0..outcome.files.len()).collect();
    order.sort_by(|&a, &b| {
        ranking
            .score(b)
            .total_cmp(&ranking.score(a))
            .then_with(|| outcome.files[a].0.rel.cmp(&outcome.files[b].0.rel))
    });

    let mut types = Vec::new();
    let mut funcs = Vec::new();
    for &fi in &order {
        let (src, parsed) = &outcome.files[fi];
        let duplicate_names = crate::budget::duplicate_symbol_names(parsed);
        for sym in &parsed.symbols {
            if opts.no_private && sym.visibility != Visibility::Public {
                continue;
            }
            let keep = matches!(
                sym.kind,
                SymbolKind::Class
                    | SymbolKind::Interface
                    | SymbolKind::Enum
                    | SymbolKind::TypeAlias
                    | SymbolKind::Function
                    | SymbolKind::Constant
            );
            if !keep {
                continue;
            }
            let entry = SymbolIndexEntry {
                anchor: crate::budget::symbol_anchor(
                    &src.rel,
                    &sym.name,
                    sym.line,
                    &duplicate_names,
                ),
                name: sym.name.clone(),
                kind: crate::render::json::kind_name(sym.kind),
                file: src.rel.clone(),
                line: sym.line,
                visibility: crate::render::json::visibility_name(sym.visibility),
            };
            if matches!(
                sym.kind,
                SymbolKind::Class
                    | SymbolKind::Interface
                    | SymbolKind::Enum
                    | SymbolKind::TypeAlias
            ) {
                types.push(entry);
            } else {
                funcs.push(entry);
            }
        }
    }
    types.extend(funcs);

    let mut map = SymbolIndexMap {
        repo_name,
        target_tokens: opts.budget,
        rendered_tokens: 0,
        total_loc: outcome.stats.total_lines,
        total_files: outcome.stats.parsed_files,
        entries: types,
        skipped_files: outcome.stats.skipped_files,
        unwired_files: outcome.stats.unwired_files,
    };
    let counter = TiktokenCounter::cl100k().map_err(MapError::Tokenizer)?;
    fit_symbol_index_map(&mut map, &counter);
    Ok(map)
}

fn fit_symbol_index_map<T: crate::budget::Tokenizer>(map: &mut SymbolIndexMap, counter: &T) {
    let candidates = map.entries.clone();
    let (mut lo, mut hi) = (0usize, candidates.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        map.entries = candidates[..mid].to_vec();
        if measure_symbol_index_map(map, counter) <= map.target_tokens {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    map.entries = candidates[..lo].to_vec();
    map.rendered_tokens = measure_symbol_index_map(map, counter);
}

fn measure_symbol_index_map<T: crate::budget::Tokenizer>(
    map: &SymbolIndexMap,
    counter: &T,
) -> usize {
    counter.count(&crate::render::markdown::render_symbol_index(map))
}

/// Resolve a symbol anchor (ADR 0009) to its signature plus its defining file's
/// one-hop file-level neighbors — the on-demand counterpart to the thin map
/// index for progressive disclosure. An anchor is `(file, name)`, optionally
/// pinned to a 1-based `line` to disambiguate overloads within a file; `file` is
/// the repo-relative path of the defining file. Returns `Ok(None)` when no such
/// declaration exists (e.g. a stale anchor), never an error. Honors
/// `opts.no_private`, `opts.langs`, and `opts.cache`; `opts.budget`/`focus` are
/// ignored (no ranking or packing). Neighbors stay file-granular by design — no
/// call resolution (PRD §3.2 non-goal).
pub fn expand_symbol(
    root: &Path,
    file: &str,
    name: &str,
    line: Option<usize>,
    opts: &MapOptions,
) -> Result<Option<SymbolExpansion>, MapError> {
    use crate::parse::Visibility;
    let (_repo_name, outcome) = discover_and_parse(root, opts)?;

    // Resolve the anchor: the file with this repo-relative path, then its first
    // symbol matching `name` (and `line`, if pinned). discover order is sorted
    // and a file's symbols are line-sorted, so "first match" is deterministic.
    let Some((file_idx, (src, parsed))) = outcome
        .files
        .iter()
        .enumerate()
        .find(|(_, (s, _))| s.rel == file)
    else {
        return Ok(None);
    };
    let Some(sym) = parsed.symbols.iter().find(|sym| {
        sym.name == name
            && line.is_none_or(|l| sym.line == l)
            && (!opts.no_private || sym.visibility == Visibility::Public)
    }) else {
        return Ok(None);
    };
    let hit = SymbolHit {
        name: sym.name.clone(),
        kind: crate::render::json::kind_name(sym.kind),
        signature: sym.signature.clone(),
        file: src.rel.clone(),
        line: sym.line,
        visibility: crate::render::json::visibility_name(sym.visibility),
    };

    // One-hop file-level neighbors from the import graph (ADR 0002): a File
    // node's index equals its index into `files`. `imports` are this file's
    // File→File out-edges; `used_by` are the files whose out-edges include it.
    let graph = crate::link::link(&outcome.files);
    let num_files = outcome.files.len();
    let mut imports: Vec<String> = graph.edges[file_idx]
        .iter()
        .filter(|&&t| t < num_files)
        .map(|&t| outcome.files[t].0.rel.clone())
        .collect();
    imports.sort();
    imports.dedup();
    let mut used_by: Vec<String> = (0..num_files)
        .filter(|&j| j != file_idx && graph.edges[j].contains(&file_idx))
        .map(|j| outcome.files[j].0.rel.clone())
        .collect();
    used_by.sort();
    used_by.dedup();

    Ok(Some(SymbolExpansion {
        hit,
        imports,
        used_by,
    }))
}

/// Shared discover → (language filter) → parse front half of the pipeline,
/// returning the repo name and parse outcome for both [`build_map`] and
/// [`find_symbol`].
fn discover_and_parse(
    root: &Path,
    opts: &MapOptions,
) -> Result<(String, crate::parse::ParseOutcome), MapError> {
    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let mut files = crate::discover::discover(root);
    if files.is_empty() {
        return Err(MapError::NoSupportedFiles);
    }
    if !opts.langs.is_empty() {
        files.retain(|f| opts.langs.contains(&f.lang));
        if files.is_empty() {
            return Err(MapError::NoMatchingLanguage);
        }
    }

    let outcome = if opts.cache {
        let mut cache = crate::cache::Cache::open(root);
        let outcome = crate::parse::parse_all_cached(files, &mut cache);
        cache.save();
        outcome
    } else {
        crate::parse::parse_all(files)
    };
    Ok((repo_name, outcome))
}

/// Resolve `--focus`-style paths to file-node seed indices for PageRank
/// personalization. A focus entry matches a file by exact repo-relative path,
/// by extension-stripped stem (`src/auth` → `src/auth.py`), or as a directory
/// prefix (`src/auth` → every `src/auth/**`). Unmatched entries are ignored
/// (an unfocused rank), mirroring the API's lenient contract. A file's index in
/// `files` equals its file-node index in the graph (ADR 0002).
fn resolve_focus_seeds(
    files: &[(crate::discover::SourceFile, crate::parse::ParsedFile)],
    focus: &[String],
) -> Vec<usize> {
    let mut seeds = Vec::new();
    for entry in focus {
        let want = entry.trim().trim_end_matches('/');
        if want.is_empty() {
            continue;
        }
        let dir_prefix = format!("{want}/");
        for (i, (src, _)) in files.iter().enumerate() {
            let rel = src.rel.as_str();
            let stem = rel.rsplit_once('.').map(|(s, _)| s).unwrap_or(rel);
            if rel == want || stem == want || rel.starts_with(&dir_prefix) {
                seeds.push(i);
            }
        }
    }
    seeds.sort_unstable();
    seeds.dedup();
    seeds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_map_on_fixtures_produces_a_map() {
        let map = build_map(
            Path::new("tests/queries/fixtures"),
            &MapOptions {
                cache: false,
                ..MapOptions::default()
            },
        )
        .expect("fixtures should map");
        assert!(!map.files.is_empty(), "expected ranked files");
        assert!(map.rendered_tokens <= map.target_tokens + map.target_tokens / 4);
        // Renders through any renderer.
        let md = crate::render::markdown::render(&map);
        assert!(md.contains("# atlas:"), "{md}");
    }

    #[test]
    fn empty_tree_is_an_error() {
        let dir = std::env::temp_dir().join(format!("atlas-api-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let err = build_map(&dir, &MapOptions::default());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(err, Err(MapError::NoSupportedFiles)));
    }

    #[test]
    fn language_filter_keeps_matching_files() {
        // The fixtures include rust.rs, so a Rust-only filter still maps.
        let map = build_map(
            Path::new("tests/queries/fixtures"),
            &MapOptions {
                langs: vec![Language::Rust],
                cache: false,
                ..MapOptions::default()
            },
        )
        .expect("rust fixture should match the rust filter");
        assert!(map.files.iter().all(|f| f.lang == "rust"));
    }

    fn find(name: &str, opts: MapOptions) -> Vec<SymbolHit> {
        find_symbol(Path::new("tests/queries/fixtures"), name, &opts).expect("fixtures parse")
    }

    #[test]
    fn find_symbol_locates_a_declaration() {
        // rust.rs defines `add` (see tests/queries/fixtures/rust.rs).
        let hits = find(
            "add",
            MapOptions {
                cache: false,
                ..MapOptions::default()
            },
        );
        assert!(!hits.is_empty(), "expected to find `add`");
        let hit = &hits[0];
        assert_eq!(hit.name, "add");
        assert!(hit.line >= 1);
        assert!(!hit.signature.is_empty());
        assert!(matches!(hit.visibility, "public" | "private"));
    }

    #[test]
    fn find_symbol_unknown_name_is_empty_not_error() {
        let hits = find(
            "definitely_not_a_real_symbol_xyz",
            MapOptions {
                cache: false,
                ..MapOptions::default()
            },
        );
        assert!(hits.is_empty());
    }

    #[test]
    fn find_symbol_results_are_deterministic_and_sorted() {
        let opts = || MapOptions {
            cache: false,
            ..MapOptions::default()
        };
        let a = find("add", opts());
        let b = find("add", opts());
        let keys: Vec<_> = a.iter().map(|h| (&h.file, h.line)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "hits must be (file, line) sorted");
        assert_eq!(
            a.iter().map(|h| (&h.file, h.line)).collect::<Vec<_>>(),
            b.iter().map(|h| (&h.file, h.line)).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn build_symbol_index_returns_budgeted_anchors_without_signatures() {
        let index = build_symbol_index(
            Path::new("tests/queries/fixtures"),
            &MapOptions {
                budget: 512,
                cache: false,
                ..MapOptions::default()
            },
        )
        .expect("fixtures should index");
        assert!(index.rendered_tokens <= index.target_tokens);
        assert!(!index.entries.is_empty(), "expected anchor entries");
        let entry = index
            .entries
            .iter()
            .find(|e| e.name == "top_level" && e.file == "rust.rs")
            .expect("rust top_level should be indexed");
        assert_eq!(entry.anchor, "rust.rs#top_level");
        assert_eq!(entry.kind, "function");
        assert!(entry.line >= 1);
    }

    fn expand(file: &str, name: &str, line: Option<usize>) -> Option<SymbolExpansion> {
        expand_symbol(
            Path::new("tests/queries/fixtures"),
            file,
            name,
            line,
            &MapOptions {
                cache: false,
                ..MapOptions::default()
            },
        )
        .expect("fixtures parse")
    }

    #[test]
    fn expand_symbol_resolves_an_anchor() {
        // rust.rs defines `top_level` (see tests/queries/fixtures/rust.rs).
        let e = expand("rust.rs", "top_level", None).expect("anchor should resolve");
        assert_eq!(e.hit.name, "top_level");
        assert_eq!(e.hit.file, "rust.rs");
        assert!(!e.hit.signature.is_empty());
        // Neighbors come back sorted + deduped (possibly empty for an isolated
        // fixture file) — the deterministic contract (NFR-4).
        let mut s = e.imports.clone();
        s.sort();
        s.dedup();
        assert_eq!(e.imports, s);
        let mut u = e.used_by.clone();
        u.sort();
        u.dedup();
        assert_eq!(e.used_by, u);
    }

    #[test]
    fn expand_symbol_unknown_anchor_is_none() {
        assert!(expand("rust.rs", "definitely_not_a_real_symbol_xyz", None).is_none());
        assert!(expand("no-such-file.rs", "top_level", None).is_none());
    }

    #[test]
    fn expand_symbol_line_pin_disambiguates() {
        let e = expand("rust.rs", "top_level", None).expect("anchor should resolve");
        let line = e.hit.line;
        // The correct line still resolves; a wrong line does not.
        assert!(expand("rust.rs", "top_level", Some(line)).is_some());
        assert!(expand("rust.rs", "top_level", Some(line + 9999)).is_none());
    }

    #[test]
    fn focus_seeds_resolve_by_path_stem_and_dir() {
        use crate::discover::SourceFile;
        use crate::parse::ParsedFile;
        use std::path::PathBuf;
        let f = |rel: &str| {
            (
                SourceFile {
                    path: PathBuf::from(rel),
                    rel: rel.to_string(),
                    lang: Language::Python,
                },
                ParsedFile::default(),
            )
        };
        let files = vec![f("src/auth.py"), f("src/auth/service.py"), f("src/util.py")];
        // Exact stem match: src/auth → src/auth.py (index 0).
        assert_eq!(
            resolve_focus_seeds(&files, &["src/auth.py".into()]),
            vec![0]
        );
        // Directory prefix: src/auth/ → src/auth/service.py (index 1); the stem
        // `src/auth` also matches src/auth.py (index 0).
        assert_eq!(
            resolve_focus_seeds(&files, &["src/auth".into()]),
            vec![0, 1]
        );
        // Unmatched focus → no seeds (unfocused rank).
        assert!(resolve_focus_seeds(&files, &["nope".into()]).is_empty());
    }
}
