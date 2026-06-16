//! Pipeline driver. Parses flags (clap derive) and runs the full M1 pipeline:
//! discover → parse → link → rank → budget → render. The default (and, in M1,
//! only) subcommand is the implicit `map`; `serve`/`diff` land in later
//! milestones. JSON/XML renderers land with FR-5's later rungs, so `--format`
//! currently accepts `md` only.

use std::path::{Path, PathBuf};

use clap::Parser;

use crate::budget::{BudgetOptions, TiktokenCounter, DEFAULT_BUDGET};
use crate::lang::Language;

#[derive(Parser, Debug)]
#[command(
    name = "repomap",
    version,
    about = "Compile a codebase into a token-budgeted structural map for LLM coding agents"
)]
pub struct Cli {
    /// Repository root to map.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Token budget for the map.
    #[arg(long, default_value_t = DEFAULT_BUDGET, value_name = "N")]
    pub budget: usize,

    /// Output format (json/xml land in later M1/M3 rungs).
    #[arg(long, value_enum, default_value_t = Format::Md)]
    pub format: Format,

    /// Restrict to languages by extension, e.g. --lang py,rs.
    #[arg(long, value_name = "CSV")]
    pub lang: Option<String>,

    /// Boost a file or directory in the ranking (repeatable),
    /// e.g. --focus src/auth --focus src/api/routes.ts.
    #[arg(long, value_name = "PATH")]
    pub focus: Vec<String>,

    /// Public API surface only — drop private symbols.
    #[arg(long)]
    pub no_private: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Markdown (default), optimized for LLM readability.
    Md,
    /// Versioned JSON schema for programmatic consumers (PRD §7.3).
    Json,
}

/// Entry point called from `main`. Exits the process with a status code.
pub fn run() {
    let cli = Cli::parse();
    std::process::exit(match run_with(cli) {
        Ok(()) => 0,
        Err(code) => code,
    })
}

fn run_with(cli: Cli) -> Result<(), i32> {
    let root = cli.path.canonicalize().map_err(|err| {
        eprintln!("repomap: cannot open {}: {err}", cli.path.display());
        2
    })?;
    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let langs = parse_langs(cli.lang.as_deref())?;
    let mut files = crate::discover::discover(&root);
    if !langs.is_empty() {
        files.retain(|f| langs.contains(&f.lang));
    }

    let mut cache = crate::cache::Cache::open(&root);
    let outcome = crate::parse::parse_all_cached(files, &mut cache);
    cache.save();
    let graph = crate::link::link(&outcome.files);
    let focus = resolve_focus(&cli.focus, &root, &outcome.files);
    let ranking = crate::rank::rank(&graph, &focus);

    let counter = TiktokenCounter::cl100k().map_err(|err| {
        eprintln!("repomap: could not initialize the tokenizer: {err}");
        1
    })?;
    let opts = BudgetOptions {
        budget_tokens: cli.budget,
        no_private: cli.no_private,
    };
    let map = crate::budget::pack(
        &outcome.files,
        &graph,
        &ranking,
        &repo_name,
        outcome.stats,
        &opts,
        &counter,
    );

    match cli.format {
        Format::Md => print!("{}", crate::render::markdown::render(&map)),
        Format::Json => print!("{}", crate::render::json::render(&map)),
    }
    Ok(())
}

/// Parse a `--lang py,rs` CSV into languages; unknown extensions are an error.
fn parse_langs(csv: Option<&str>) -> Result<Vec<Language>, i32> {
    let Some(csv) = csv else {
        return Ok(Vec::new());
    };
    let mut langs = Vec::new();
    for token in csv.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        match Language::from_extension(token) {
            Some(lang) => langs.push(lang),
            None => {
                eprintln!("repomap: unknown language extension {token:?}");
                return Err(2);
            }
        }
    }
    Ok(langs)
}

/// Map `--focus` paths (files or directories, interpreted relative to the
/// repo root) to file-node indices for the PageRank personalization vector.
/// A File node's index equals its index into `files` (link ADR-0002), so the
/// returned indices are directly usable as personalization seeds. Focus
/// targets that don't resolve under the root are skipped.
fn resolve_focus(
    focus: &[String],
    root: &Path,
    files: &[(crate::discover::SourceFile, crate::parse::ParsedFile)],
) -> Vec<usize> {
    let mut seeds = Vec::new();
    for target in focus {
        let Some(rel) = canonical_rel(target, root) else {
            eprintln!("repomap: --focus path not found under the repo: {target:?}");
            continue;
        };
        let dir_prefix = format!("{rel}/");
        for (i, (src, _)) in files.iter().enumerate() {
            if src.rel == rel || src.rel.starts_with(&dir_prefix) {
                seeds.push(i);
            }
        }
    }
    seeds.sort_unstable();
    seeds.dedup();
    seeds
}

/// Resolve a focus target (relative to `root`) to a root-relative, `/`-joined
/// path key, or `None` if it doesn't exist on disk under the root.
fn canonical_rel(target: &str, root: &Path) -> Option<String> {
    let canon = root.join(target).canonicalize().ok()?;
    let rel = canon.strip_prefix(root).ok()?;
    Some(
        rel.to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("valid args")
    }

    #[test]
    fn defaults() {
        let cli = parse(&["repomap"]);
        assert_eq!(cli.path, PathBuf::from("."));
        assert_eq!(cli.budget, DEFAULT_BUDGET);
        assert_eq!(cli.format, Format::Md);
        assert!(cli.focus.is_empty());
        assert!(!cli.no_private);
        assert!(cli.lang.is_none());
    }

    #[test]
    fn parses_flags() {
        let cli = parse(&[
            "repomap",
            "../somewhere",
            "--budget",
            "4096",
            "--lang",
            "py,rs",
            "--focus",
            "src/auth",
            "--focus",
            "src/api/routes.ts",
            "--no-private",
        ]);
        assert_eq!(cli.path, PathBuf::from("../somewhere"));
        assert_eq!(cli.budget, 4096);
        assert_eq!(cli.lang.as_deref(), Some("py,rs"));
        assert_eq!(cli.focus, vec!["src/auth", "src/api/routes.ts"]);
        assert!(cli.no_private);
    }

    #[test]
    fn accepts_known_formats_rejects_bad_input() {
        assert!(Cli::try_parse_from(["repomap", "--budget", "notanumber"]).is_err());
        assert_eq!(parse(&["repomap", "--format", "json"]).format, Format::Json);
        assert_eq!(parse(&["repomap", "--format", "md"]).format, Format::Md);
        // xml lands in M3.
        assert!(Cli::try_parse_from(["repomap", "--format", "xml"]).is_err());
    }

    #[test]
    fn parse_langs_csv() {
        assert_eq!(
            parse_langs(Some("py,rs")).unwrap(),
            vec![Language::Python, Language::Rust]
        );
        assert_eq!(parse_langs(None).unwrap(), Vec::<Language>::new());
        assert_eq!(parse_langs(Some("cobol")).err(), Some(2));
    }
}
