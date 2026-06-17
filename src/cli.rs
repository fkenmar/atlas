//! Pipeline driver. Parses flags (clap derive) and runs the full M1 pipeline:
//! discover → parse → link → rank → budget → render. The default (and, in M1,
//! only) subcommand is the implicit `map`; `serve`/`diff` land in later
//! milestones.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser};
use clap_complete::Shell;

use crate::budget::{BudgetOptions, TiktokenCounter, DEFAULT_BUDGET};
use crate::lang::Language;

const EXAMPLES: &str = "\
EXAMPLES:
  atlas .                      Map the current folder (default 2,048-token budget)
  atlas . --budget 4096        Give the map a larger token budget
  atlas src --focus src/auth   Rank files under src/auth higher
  atlas . --no-private         Public API surface only
  atlas . --format json        Emit JSON instead of Markdown
  atlas . > map.md             Save the map (e.g. to feed an agent)
  atlas --completions zsh      Print a shell completion script (bash/zsh/fish/…)

Pipe the output into your AI coding agent's context so it can navigate the repo
without reading every file. Docs: https://github.com/fkenmar/atlas";

#[derive(Parser, Debug)]
#[command(
    name = "atlas",
    version,
    about = "Compile a codebase into a token-budgeted structural map for LLM coding agents",
    long_about = "Walks a repository, extracts every signature, type, and import edge \
(never function bodies), ranks files by how central they are to the codebase, packs the \
most important files into a token budget, and prints a Markdown (or JSON) map to stdout — \
ready to drop into an LLM coding agent's context.",
    after_help = EXAMPLES
)]
pub struct Cli {
    /// Repository root to map.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Token budget for the map.
    #[arg(short, long, default_value_t = DEFAULT_BUDGET, value_name = "N")]
    pub budget: usize,

    /// Output format: md (default) or json.
    #[arg(short, long, value_enum, default_value_t = Format::Md)]
    pub format: Format,

    /// Restrict to languages by extension, e.g. --lang py,rs.
    #[arg(short, long, value_name = "CSV")]
    pub lang: Option<String>,

    /// Boost a file or directory in the ranking. Repeatable, and accepts a
    /// comma-separated list: --focus src/auth,src/api or --focus src/auth.
    #[arg(long, value_name = "PATHS")]
    pub focus: Vec<String>,

    /// Public API surface only — drop private symbols.
    #[arg(long)]
    pub no_private: bool,

    /// Colorize Markdown output: auto (default, only when writing to a
    /// terminal), always, or never. Piped output is never colored.
    #[arg(long, value_enum, default_value_t = Color::Auto, value_name = "WHEN")]
    pub color: Color,

    /// Print a shell completion script to stdout and exit, e.g.
    /// `atlas --completions zsh > ~/.zfunc/_atlas`.
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Color only when stdout is a terminal (and `NO_COLOR` is unset).
    Auto,
    /// Always color, even when piped.
    Always,
    /// Never color.
    Never,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Markdown (default), optimized for LLM readability.
    Md,
    /// Versioned, stable JSON schema for programmatic consumers.
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
    // `--completions <shell>`: print the script and exit before doing any work,
    // so it's usable in any context (no repo needed).
    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "atlas", &mut std::io::stdout());
        return Ok(());
    }

    if cli.budget == 0 {
        eprintln!("atlas: --budget must be at least 1 token (the default is {DEFAULT_BUDGET})");
        return Err(2);
    }

    // `serve`/`diff` are promoted in the README as planned commands; a user who
    // tries `atlas serve` lands here (clap parses the word as the path). Give a
    // clear "planned" message instead of a path-not-found error.
    let path_str = cli.path.to_string_lossy();
    if matches!(path_str.as_ref(), "serve" | "diff") && !cli.path.exists() {
        eprintln!(
            "atlas: '{path_str}' is a planned command, not available yet — \
             see the roadmap: https://github.com/fkenmar/atlas#project-status"
        );
        return Err(2);
    }

    let root = cli.path.canonicalize().map_err(|err| {
        match err.kind() {
            std::io::ErrorKind::NotFound => eprintln!(
                "atlas: path not found: {} — atlas maps a directory; pass a repo root, \
                 or omit the path to map the current folder",
                cli.path.display()
            ),
            std::io::ErrorKind::PermissionDenied => {
                eprintln!("atlas: permission denied reading {}", cli.path.display())
            }
            _ => eprintln!("atlas: cannot open {}: {err}", cli.path.display()),
        }
        2
    })?;

    if !root.is_dir() {
        eprintln!(
            "atlas: {} is not a directory — atlas maps a repository root, not a single file",
            cli.path.display()
        );
        return Err(2);
    }

    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let langs = parse_langs(cli.lang.as_deref())?;
    let mut files = crate::discover::discover(&root);
    let discovered = files.len();
    if discovered == 0 {
        eprintln!(
            "atlas: no supported source files found under {}. atlas maps Python, \
             TypeScript/JavaScript, and Rust — is this the repo root? (see --help)",
            cli.path.display()
        );
        return Err(1);
    }
    if !langs.is_empty() {
        files.retain(|f| langs.contains(&f.lang));
        if files.is_empty() {
            eprintln!(
                "atlas: --lang matched none of the {discovered} source file(s) found under {}",
                cli.path.display()
            );
            return Err(1);
        }
    }

    let mut cache = crate::cache::Cache::open(&root);
    let outcome = crate::parse::parse_all_cached(files, &mut cache);
    cache.save();
    let graph = crate::link::link(&outcome.files);
    let focus = resolve_focus(&cli.focus, &root, &outcome.files)?;
    let ranking = crate::rank::rank(&graph, &focus);

    let counter = TiktokenCounter::cl100k().map_err(|err| {
        eprintln!("atlas: could not initialize the tokenizer: {err}");
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
        Format::Md => {
            let md = crate::render::markdown::render(&map);
            if should_color(cli.color) {
                print!("{}", crate::render::color::colorize(&md));
            } else {
                print!("{md}");
            }
        }
        // JSON is structured data for programmatic consumers — never colorized.
        Format::Json => print!("{}", crate::render::json::render(&map)),
    }
    Ok(())
}

/// Decide whether to ANSI-colorize Markdown output. `auto` colors only when
/// stdout is a terminal and `NO_COLOR` (https://no-color.org) is unset; an
/// explicit `--color always` overrides both.
fn should_color(when: Color) -> bool {
    match when {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal(),
    }
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
                eprintln!(
                    "atlas: unknown --lang value {token:?}. Supported extensions: \
                     py, pyi, ts, tsx, js, jsx, mjs, cjs, rs (Python, TypeScript/JavaScript, Rust)"
                );
                return Err(2);
            }
        }
    }
    Ok(langs)
}

/// Map `--focus` paths (files or directories, relative to the repo root) to
/// file-node indices for the PageRank personalization vector. Each `--focus`
/// value may itself be a comma-separated list (mirroring `--lang`). A File
/// node's index equals its index into `files` (link ADR-0002). If *every*
/// focus path fails to resolve, that's an error (exit 2) rather than a
/// silently-unfocused map; if only some fail, they're warned and skipped.
fn resolve_focus(
    focus: &[String],
    root: &Path,
    files: &[(crate::discover::SourceFile, crate::parse::ParsedFile)],
) -> Result<Vec<usize>, i32> {
    if focus.is_empty() {
        return Ok(Vec::new());
    }
    let targets: Vec<String> = focus
        .iter()
        .flat_map(|f| f.split(','))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let mut seeds = Vec::new();
    let mut unresolved = Vec::new();
    for target in &targets {
        let Some(rel) = canonical_rel(target, root) else {
            unresolved.push(target.clone());
            continue;
        };
        let dir_prefix = format!("{rel}/");
        for (i, (src, _)) in files.iter().enumerate() {
            if src.rel == rel || src.rel.starts_with(&dir_prefix) {
                seeds.push(i);
            }
        }
    }

    if seeds.is_empty() && !unresolved.is_empty() {
        eprintln!(
            "atlas: none of the --focus paths exist under the repo root \
             (paths are relative to it): {}",
            unresolved.join(", ")
        );
        return Err(2);
    }
    if !unresolved.is_empty() {
        eprintln!(
            "atlas: warning: ignored --focus path(s) not found under the repo root: {}",
            unresolved.join(", ")
        );
    }
    seeds.sort_unstable();
    seeds.dedup();
    Ok(seeds)
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
        let cli = parse(&["atlas"]);
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
            "atlas",
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
    fn short_flags_work() {
        let cli = parse(&["atlas", "-b", "512", "-f", "json", "-l", "rs"]);
        assert_eq!(cli.budget, 512);
        assert_eq!(cli.format, Format::Json);
        assert_eq!(cli.lang.as_deref(), Some("rs"));
    }

    #[test]
    fn color_and_completions_parse() {
        assert_eq!(parse(&["atlas"]).color, Color::Auto);
        assert_eq!(parse(&["atlas", "--color", "never"]).color, Color::Never);
        assert_eq!(parse(&["atlas", "--color", "always"]).color, Color::Always);
        assert!(parse(&["atlas"]).completions.is_none());
        assert_eq!(
            parse(&["atlas", "--completions", "zsh"]).completions,
            Some(Shell::Zsh)
        );
        // An unknown shell is rejected.
        assert!(Cli::try_parse_from(["atlas", "--completions", "tcsh"]).is_err());
    }

    #[test]
    fn accepts_known_formats_rejects_bad_input() {
        assert!(Cli::try_parse_from(["atlas", "--budget", "notanumber"]).is_err());
        assert_eq!(parse(&["atlas", "--format", "json"]).format, Format::Json);
        assert_eq!(parse(&["atlas", "--format", "md"]).format, Format::Md);
        // xml lands in M3.
        assert!(Cli::try_parse_from(["atlas", "--format", "xml"]).is_err());
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
