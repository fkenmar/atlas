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
