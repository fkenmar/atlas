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
  atlas diff old/ new/         Structural delta between two trees (added/removed/changed)
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
most important files into a token budget, and prints a Markdown (or JSON/XML) map to stdout — \
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

    /// Output format: md (default), json, or xml.
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
    /// Well-formed XML for prompt-injection-safe wrapping in Claude prompts.
    Xml,
}

/// `atlas diff <old> <new>` — structural delta between two trees (ADR 0005).
/// Routed by [`run`] before the map parser, git-style, so it stays out of the
/// flat map `Cli`.
#[derive(Parser, Debug)]
#[command(
    name = "atlas diff",
    about = "Structural diff of the map between two trees"
)]
pub struct DiffArgs {
    /// The old / base tree.
    pub old: PathBuf,
    /// The new / changed tree.
    pub new: PathBuf,
    /// Output format: md (default), json, or xml.
    #[arg(short, long, value_enum, default_value_t = Format::Md)]
    pub format: Format,
    /// Restrict to languages by extension, e.g. --lang py,rs.
    #[arg(short, long, value_name = "CSV")]
    pub lang: Option<String>,
    /// Public API surface only — drop private symbols before comparing.
    #[arg(long)]
    pub no_private: bool,
}

/// Entry point called from `main`. Exits the process with a status code.
pub fn run() {
    // Git-style dispatch: `atlas diff <old> <new>` routes to the diff command
    // before the map parser sees the args (ADR 0005); everything else is the
    // implicit `map` command.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("diff") {
        let diff = DiffArgs::parse_from(
            std::iter::once("atlas diff".to_string()).chain(args.into_iter().skip(2)),
        );
        std::process::exit(match run_diff(diff) {
            Ok(()) => 0,
            Err(code) => code,
        });
    }
    let cli = Cli::parse();
    std::process::exit(match run_with(cli) {
        Ok(()) => 0,
        Err(code) => code,
    })
}

/// `atlas diff <old> <new>`: parse both trees fully (no rank/budget) and print
/// the structural delta (ADR 0005). Exit 0 whether or not anything changed —
/// the diff is informational (PRD open question on CI-gating left for later).
fn run_diff(args: DiffArgs) -> Result<(), i32> {
    let langs = parse_langs(args.lang.as_deref())?;
    let old_files = discover_tree(&args.old, &langs)?;
    let new_files = discover_tree(&args.new, &langs)?;
    let old_outcome = crate::parse::parse_all(old_files);
    let new_outcome = crate::parse::parse_all(new_files);
    let opts = crate::diff::DiffOptions {
        no_private: args.no_private,
    };
    let delta = crate::diff::diff(&old_outcome.files, &new_outcome.files, &opts);
    let old_label = args.old.display().to_string();
    let new_label = args.new.display().to_string();
    let (os, ns) = (&old_outcome.stats, &new_outcome.stats);
    let out = match args.format {
        Format::Md => crate::render::diff::render(&delta, &old_label, &new_label, os, ns),
        Format::Json => crate::render::diff::render_json(&delta, &old_label, &new_label, os, ns),
        Format::Xml => crate::render::diff::render_xml(&delta, &old_label, &new_label, os, ns),
    };
    print!("{out}");
    Ok(())
}

/// Canonicalize `path` and confirm it's a directory, with actionable error
/// messages. Shared by the map and diff commands so a bad path reports the same
/// helpful guidance either way (NotFound / PermissionDenied / not-a-directory).
fn canonicalize_root(path: &Path) -> Result<PathBuf, i32> {
    let root = path.canonicalize().map_err(|err| {
        match err.kind() {
            std::io::ErrorKind::NotFound => eprintln!(
                "atlas: path not found: {} — atlas maps a directory; pass a repo root, \
                 or omit the path to map the current folder",
                path.display()
            ),
            std::io::ErrorKind::PermissionDenied => {
                eprintln!("atlas: permission denied reading {}", path.display())
            }
            _ => eprintln!("atlas: cannot open {}: {err}", path.display()),
        }
        2
    })?;
    if !root.is_dir() {
        eprintln!(
            "atlas: {} is not a directory — atlas maps a repository root, not a single file",
            path.display()
        );
        return Err(2);
    }
    Ok(root)
}

/// Discover + language-filter one side of a diff. An empty tree is allowed (it
/// diffs as all-added or all-removed); a missing path or non-directory is an
/// error.
fn discover_tree(path: &Path, langs: &[Language]) -> Result<Vec<crate::discover::SourceFile>, i32> {
    let root = canonicalize_root(path)?;
    let mut files = crate::discover::discover(&root);
    if !langs.is_empty() {
        files.retain(|f| langs.contains(&f.lang));
    }
    Ok(files)
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

    // `serve` is promoted in the README as a planned command; a user who tries
    // `atlas serve` lands here (clap parses the word as the path). Give a clear
    // "planned" message instead of a path-not-found error. (`diff` is a real
    // command now, routed in `run` before parsing — see ADR 0005.)
    let path_str = cli.path.to_string_lossy();
    if matches!(path_str.as_ref(), "serve") && !cli.path.exists() {
        eprintln!(
            "atlas: '{path_str}' is a planned command, not available yet — \
             see the roadmap: https://github.com/fkenmar/atlas#project-status"
        );
        return Err(2);
    }

    let root = canonicalize_root(&cli.path)?;

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
        // JSON and XML are structured data for programmatic consumers — never
        // colorized.
        Format::Json => print!("{}", crate::render::json::render(&map)),
        Format::Xml => print!("{}", crate::render::xml::render(&map)),
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
        assert_eq!(parse(&["atlas", "--format", "xml"]).format, Format::Xml);
        assert!(Cli::try_parse_from(["atlas", "--format", "bogus"]).is_err());
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

    #[test]
    fn diff_args_parse_old_and_new() {
        // argv[0] mirrors the synthetic name the `diff` router passes.
        let d = DiffArgs::parse_from(["atlas diff", "old", "new"]);
        assert_eq!(d.old, PathBuf::from("old"));
        assert_eq!(d.new, PathBuf::from("new"));
        assert!(!d.no_private);
        assert_eq!(d.lang, None);
    }

    #[test]
    fn diff_args_accept_flags() {
        let d = DiffArgs::parse_from(["atlas diff", "a", "b", "--lang", "py", "--no-private"]);
        assert_eq!(d.lang.as_deref(), Some("py"));
        assert!(d.no_private);
    }

    #[test]
    fn diff_args_format_defaults_md_and_parses() {
        assert_eq!(
            DiffArgs::parse_from(["atlas diff", "a", "b"]).format,
            Format::Md
        );
        assert_eq!(
            DiffArgs::parse_from(["atlas diff", "a", "b", "--format", "json"]).format,
            Format::Json
        );
        assert_eq!(
            DiffArgs::parse_from(["atlas diff", "a", "b", "-f", "xml"]).format,
            Format::Xml
        );
    }

    #[test]
    fn diff_requires_two_paths() {
        assert!(DiffArgs::try_parse_from(["atlas diff", "only-one"]).is_err());
    }
}
