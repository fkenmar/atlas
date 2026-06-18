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

    let graph = crate::link::link(&outcome.files);
    let ranking = crate::rank::rank(&graph, &[]);
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
}
