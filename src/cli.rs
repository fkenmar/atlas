//! Pipeline driver. Parses flags (clap derive) and runs the full map pipeline:
//! discover → parse → link → rank → budget → render. Map mode is the implicit
//! default; `diff` and `serve --mcp` are routed git-style before map parsing.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
  atlas . -o atlas-map.md      Atomically write the map to a file
  atlas . --for-agent          Add a short agent-facing Markdown preamble
  atlas . --timings            Print stage timings to stderr
  atlas diff HEAD~1 HEAD       Structural delta between two git revisions (or two dirs)
  atlas . > map.md             Save the map (e.g. to feed an agent)
  atlas --completions zsh      Print a shell completion script (bash/zsh/fish/…)

Pipe the output into your AI coding agent's context so it can navigate the repo
without reading every file. Docs: https://github.com/fkenmar/atlas";

const SUPPORTED_LANGUAGE_SUMMARY: &str = "Python, TypeScript/JavaScript, Rust, Go, Java, and C/C++";
const SUPPORTED_EXTENSION_SUMMARY: &str = "\
py, pyi, ts, tsx, mts, cts, js, jsx, mjs, cjs, rs, go, java, c, h, cc, cpp, cxx, hpp, hh";
const AGENT_PREAMBLE: &str = "\
> atlas agent note: use this map as a navigation index before opening files.
> It contains signatures, types, imports, and reverse dependencies only; inspect source before editing implementation.

";
const EXTENSION_HINT_SKIP_DIRS: &[&str] = &[
    ".git",
    ".atlas",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "venv",
    "env",
    "__pycache__",
    "site-packages",
    "vendor",
    "third_party",
    "coverage",
];
const MAX_EXTENSION_HINT_FILES: usize = 2_000;

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

    /// Atomically write output to this file instead of stdout.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

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

    /// Add a short Markdown preamble telling an agent how to use the map.
    #[arg(long)]
    pub for_agent: bool,

    /// Print pipeline stage timings to stderr.
    #[arg(long)]
    pub timings: bool,

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

/// `atlas serve --mcp` — run atlas as a server (ADR 0008). Routed by [`run`]
/// before the map parser, git-style.
#[derive(Parser, Debug)]
#[command(name = "atlas serve", about = "Run atlas as a server")]
pub struct ServeArgs {
    /// Run as an MCP server over stdio (newline-delimited JSON-RPC) so an agent
    /// can pull a fresh map as a tool call.
    #[arg(long)]
    pub mcp: bool,
    /// Confine `get_map` to this directory (default: current directory). A
    /// requested path outside it is rejected (#102).
    #[arg(long, value_name = "DIR")]
    pub root: Option<PathBuf>,
}

/// `atlas cache <info|clean>` — inspect or clear the parse cache (#80). Routed
/// by [`run`] before the map parser, git-style.
#[derive(Parser, Debug)]
#[command(name = "atlas cache", about = "Inspect or clear the parse cache")]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum CacheCommand {
    /// Report cache path, size, entry count, version, and self-ignore status.
    Info {
        /// Repository root (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Delete the cache file. Requires --force (the cache is safe to rebuild).
    Clean {
        /// Repository root (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Actually delete (without this, prints what would be removed).
        #[arg(long)]
        force: bool,
    },
}

/// `atlas explain <path>` — explain why a file ranks where it does (#94). Routed
/// git-style, kept out of the flat map `Cli`.
#[derive(Parser, Debug)]
#[command(
    name = "atlas explain",
    about = "Explain a file's rank (importers, imports, score)"
)]
pub struct ExplainArgs {
    /// File whose rank to explain.
    pub path: PathBuf,
    /// Repository root to rank within (default: current directory).
    #[arg(long, value_name = "DIR")]
    pub root: Option<PathBuf>,
    /// Boost a file or directory in the ranking (to see its effect on `path`).
    #[arg(long, value_name = "PATHS")]
    pub focus: Vec<String>,
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
    // `atlas serve --mcp` runs the stdio MCP server (ADR 0008).
    if args.get(1).map(String::as_str) == Some("serve") {
        let serve = ServeArgs::parse_from(
            std::iter::once("atlas serve".to_string()).chain(args.into_iter().skip(2)),
        );
        std::process::exit(match run_serve(serve) {
            Ok(()) => 0,
            Err(code) => code,
        });
    }
    // `atlas cache <info|clean>` inspects or clears the parse cache (#80).
    if args.get(1).map(String::as_str) == Some("cache") {
        let cache = CacheArgs::parse_from(
            std::iter::once("atlas cache".to_string()).chain(args.into_iter().skip(2)),
        );
        std::process::exit(match run_cache(cache) {
            Ok(()) => 0,
            Err(code) => code,
        });
    }
    // `atlas explain <path>` explains a file's rank (#94).
    if args.get(1).map(String::as_str) == Some("explain") {
        let explain = ExplainArgs::parse_from(
            std::iter::once("atlas explain".to_string()).chain(args.into_iter().skip(2)),
        );
        std::process::exit(match run_explain(explain) {
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

/// `atlas diff <old> <new>`: each side is a directory or a git revision (ADR
/// 0005/0007); parse both trees fully (no rank/budget) and print the structural
/// delta. Exit 0 whether or not anything changed — the diff is informational
/// (PRD open question on CI-gating left for later).
fn run_diff(args: DiffArgs) -> Result<(), i32> {
    let langs = parse_langs(args.lang.as_deref())?;
    let old_label = args.old.to_string_lossy().into_owned();
    let new_label = args.new.to_string_lossy().into_owned();

    let old_tree = resolve_tree(&args.old, 0)?;
    // Clean up the first worktree if resolving the second fails.
    let new_tree = match resolve_tree(&args.new, 1) {
        Ok(t) => t,
        Err(code) => {
            old_tree.cleanup();
            return Err(code);
        }
    };

    let render = || -> String {
        let mut old_files = crate::discover::discover(&old_tree.root);
        let mut new_files = crate::discover::discover(&new_tree.root);
        if !langs.is_empty() {
            old_files.retain(|f| langs.contains(&f.lang));
            new_files.retain(|f| langs.contains(&f.lang));
        }
        let old_outcome = crate::parse::parse_all(old_files);
        let new_outcome = crate::parse::parse_all(new_files);
        let opts = crate::diff::DiffOptions {
            no_private: args.no_private,
        };
        let delta = crate::diff::diff(&old_outcome.files, &new_outcome.files, &opts);
        let (os, ns) = (&old_outcome.stats, &new_outcome.stats);
        match args.format {
            Format::Md => crate::render::diff::render(&delta, &old_label, &new_label, os, ns),
            Format::Json => {
                crate::render::diff::render_json(&delta, &old_label, &new_label, os, ns)
            }
            Format::Xml => crate::render::diff::render_xml(&delta, &old_label, &new_label, os, ns),
        }
    };
    let out = render();
    old_tree.cleanup();
    new_tree.cleanup();
    print!("{out}");
    Ok(())
}

fn run_serve(args: ServeArgs) -> Result<(), i32> {
    if !args.mcp {
        eprintln!("atlas: serve currently supports only --mcp");
        return Err(2);
    }
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    crate::mcp::serve(&root).map_err(|err| {
        eprintln!("atlas: MCP server failed: {err}");
        1
    })
}

/// `atlas cache info|clean` — inspect or clear the parse cache (#80).
fn run_cache(args: CacheArgs) -> Result<(), i32> {
    match args.command {
        CacheCommand::Info { path } => {
            let info = crate::cache::info(&path);
            println!("cache:         {}", info.path.display());
            if info.exists {
                println!("status:        present");
                println!("size:          {} bytes", info.size_bytes);
                println!("entries:       {}", info.entries);
            } else {
                println!("status:        absent (no cache written yet)");
            }
            println!("cache version: {}", info.version);
            println!(
                "self-ignored:  {}",
                if info.self_ignored {
                    "yes (.atlas/.gitignore present)"
                } else {
                    "no"
                }
            );
            Ok(())
        }
        CacheCommand::Clean { path, force } => {
            let cache_path = path.join(".atlas").join("cache");
            if !force {
                eprintln!(
                    "atlas cache clean: would remove {} — re-run with --force to delete",
                    cache_path.display()
                );
                return Err(2);
            }
            match crate::cache::clean(&path) {
                Ok(true) => {
                    println!("removed {}", cache_path.display());
                    Ok(())
                }
                Ok(false) => {
                    println!("no cache to remove at {}", cache_path.display());
                    Ok(())
                }
                Err(err) => {
                    eprintln!("atlas cache clean: {err}");
                    Err(1)
                }
            }
        }
    }
}

/// `atlas explain <path>` — report why a file ranks where it does (#94):
/// rank position, PageRank score, importer/import counts, and focus boost.
/// Read-only (uncached parse); leaves normal map output untouched.
fn run_explain(args: ExplainArgs) -> Result<(), i32> {
    let root = canonicalize_root(&args.root.clone().unwrap_or_else(|| PathBuf::from(".")))?;
    let files = crate::discover::discover(&root);
    if files.is_empty() {
        eprintln!(
            "atlas: no supported source files found under {}",
            root.display()
        );
        return Err(1);
    }
    let outcome = crate::parse::parse_all(files);
    let num_files = outcome.files.len();
    let graph = crate::link::link(&outcome.files);
    let focus = resolve_focus(&args.focus, &root, &outcome.files)?;
    let ranking = crate::rank::rank(&graph, &focus);

    let target = args.path.to_string_lossy();
    let Some(rel) = canonical_rel(&target, &root) else {
        eprintln!(
            "atlas explain: {} is not inside {}",
            args.path.display(),
            root.display()
        );
        return Err(2);
    };
    let Some(idx) = outcome.files.iter().position(|(s, _)| s.rel == rel) else {
        eprintln!(
            "atlas explain: {rel} is not a mapped source file under {}",
            root.display()
        );
        return Err(1);
    };

    // Rank position among files: by score desc, ties broken by rel (NFR-4).
    let mut ranked: Vec<usize> = (0..num_files).collect();
    ranked.sort_by(|&a, &b| {
        ranking
            .score(b)
            .partial_cmp(&ranking.score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| outcome.files[a].0.rel.cmp(&outcome.files[b].0.rel))
    });
    let position = ranked.iter().position(|&i| i == idx).map_or(0, |p| p + 1);

    // File node index == file index (link.rs §1). Importers = nodes pointing at
    // it; imports = its edges to other File nodes.
    let importers = (0..graph.nodes.len())
        .filter(|&j| graph.edges[j].contains(&idx))
        .count();
    let imports = graph.edges[idx].iter().filter(|&&t| t < num_files).count();

    println!("rank explanation for {rel}");
    println!("  rank:      #{position} of {num_files} files");
    println!("  score:     {:.6}", ranking.score(idx));
    println!("  importers: {importers} (references into this file)");
    println!("  imports:   {imports} (other files it imports)");
    println!(
        "  focus:     {}",
        if focus.contains(&idx) {
            "boosted (--focus)"
        } else {
            "not boosted"
        }
    );
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

/// One side of a diff resolved to a directory on disk: either an existing path,
/// or a git revision checked out into a temp worktree that must be cleaned up.
struct ResolvedTree {
    root: PathBuf,
    /// Temp worktree to remove when done (`None` for a plain path).
    worktree: Option<PathBuf>,
}

impl ResolvedTree {
    /// Best-effort removal of a materialized worktree (no-op for a plain path).
    fn cleanup(&self) {
        if let Some(wt) = &self.worktree {
            let _ = std::process::Command::new("git")
                .args(["worktree", "remove", "--force"])
                .arg(wt)
                .output();
        }
    }
}

/// Resolve a diff argument (ADR 0007): an existing directory is used as-is; any
/// other string is treated as a git revision and checked out into a temp
/// worktree. `index` distinguishes the two sides' temp dirs.
fn resolve_tree(arg: &Path, index: usize) -> Result<ResolvedTree, i32> {
    if arg.is_dir() {
        return Ok(ResolvedTree {
            root: canonicalize_root(arg)?,
            worktree: None,
        });
    }
    materialize_revision(&arg.to_string_lossy(), index)
}

/// Check out a git revision into a fresh temp worktree via the `git` CLI (no
/// git2 dependency — ADR 0007). Errors cleanly if `rev` isn't a commit or git
/// can't run.
fn materialize_revision(rev: &str, index: usize) -> Result<ResolvedTree, i32> {
    let verified = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{rev}^{{commit}}"))
        .output();
    match verified {
        Ok(o) if o.status.success() => {}
        Ok(_) => {
            eprintln!(
                "atlas: '{rev}' is not a directory or a git revision — pass a directory, \
                 or run inside a git repo with a valid revision (e.g. HEAD~1)"
            );
            return Err(2);
        }
        Err(e) => {
            eprintln!("atlas: git is required to diff revisions, but could not be run: {e}");
            return Err(2);
        }
    }

    let dir = std::env::temp_dir().join(format!("atlas-diff-{}-{index}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); // clear any stale dir from a prior crash
    let added = std::process::Command::new("git")
        .args(["worktree", "add", "--detach", "--quiet"])
        .arg(&dir)
        .arg(rev)
        .output();
    match added {
        Ok(o) if o.status.success() => Ok(ResolvedTree {
            root: dir.clone(),
            worktree: Some(dir),
        }),
        Ok(o) => {
            eprintln!(
                "atlas: could not check out revision '{rev}': {}",
                String::from_utf8_lossy(&o.stderr).trim()
            );
            Err(2)
        }
        Err(e) => {
            eprintln!("atlas: git is required to diff revisions, but could not be run: {e}");
            Err(2)
        }
    }
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

    if cli.for_agent && cli.format != Format::Md {
        eprintln!("atlas: --for-agent is only supported with Markdown output (--format md)");
        return Err(2);
    }

    // Defensive fallback for direct `run_with` callers: real CLI invocations of
    // `atlas serve ...` are routed in `run` before map parsing.
    let path_str = cli.path.to_string_lossy();
    if matches!(path_str.as_ref(), "serve") && !cli.path.exists() {
        eprintln!("atlas: '{path_str}' is a command, not a repo path — use `atlas serve --mcp`");
        return Err(2);
    }

    let total_start = Instant::now();
    let root = canonicalize_root(&cli.path)?;

    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let langs = parse_langs(cli.lang.as_deref())?;
    let discover_start = Instant::now();
    let mut files = crate::discover::discover(&root);
    let discovered = files.len();
    log_timing(
        cli.timings,
        "discover",
        discover_start.elapsed(),
        &format!("{discovered} source file(s)"),
    );
    if discovered == 0 {
        let hint = unsupported_extension_hint(&root)
            .map(|h| format!(" Detected file extension(s): {h}."))
            .unwrap_or_default();
        eprintln!(
            "atlas: no supported source files found under {}. atlas maps \
             {SUPPORTED_LANGUAGE_SUMMARY} ({SUPPORTED_EXTENSION_SUMMARY}).{hint} \
             Is this the repo root? (see --help)",
            cli.path.display(),
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

    let parse_start = Instant::now();
    let mut cache = crate::cache::Cache::open(&root);
    let outcome = crate::parse::parse_all_cached(files, &mut cache);
    cache.save();
    log_timing(
        cli.timings,
        "parse/cache",
        parse_start.elapsed(),
        &format!(
            "{} parsed, {} skipped, {} unwired, {} LOC",
            outcome.stats.parsed_files,
            outcome.stats.skipped_files,
            outcome.stats.unwired_files,
            outcome.stats.total_lines
        ),
    );

    let link_start = Instant::now();
    let graph = crate::link::link(&outcome.files);
    log_timing(
        cli.timings,
        "link",
        link_start.elapsed(),
        &format!("{} file node(s)", outcome.files.len()),
    );

    let rank_start = Instant::now();
    let focus = resolve_focus(&cli.focus, &root, &outcome.files)?;
    let ranking = crate::rank::rank(&graph, &focus);
    log_timing(
        cli.timings,
        "rank",
        rank_start.elapsed(),
        &format!("{} focus seed(s)", focus.len()),
    );

    let budget_start = Instant::now();
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
    log_timing(
        cli.timings,
        "budget",
        budget_start.elapsed(),
        &format!("{} rendered token(s)", map.rendered_tokens),
    );

    let render_start = Instant::now();
    let output = match cli.format {
        Format::Md => {
            let mut md = crate::render::markdown::render(&map);
            if cli.for_agent {
                md = with_agent_preamble(&md);
            }
            if cli.output.is_none() && should_color(cli.color) {
                crate::render::color::colorize(&md)
            } else {
                md
            }
        }
        // JSON and XML are structured data for programmatic consumers — never
        // colorized.
        Format::Json => crate::render::json::render(&map),
        Format::Xml => crate::render::xml::render(&map),
    };
    log_timing(
        cli.timings,
        "render",
        render_start.elapsed(),
        &format!("{} byte(s)", output.len()),
    );

    let write_start = Instant::now();
    if let Some(path) = cli.output.as_deref() {
        write_output_atomic(path, &output)?;
    } else {
        print!("{output}");
    }
    log_timing(
        cli.timings,
        "write",
        write_start.elapsed(),
        cli.output
            .as_deref()
            .and_then(Path::to_str)
            .unwrap_or("stdout"),
    );
    log_timing(cli.timings, "total", total_start.elapsed(), "");
    Ok(())
}

fn with_agent_preamble(md: &str) -> String {
    let mut out = String::with_capacity(AGENT_PREAMBLE.len() + md.len());
    out.push_str(AGENT_PREAMBLE);
    out.push_str(md);
    out
}

fn log_timing(enabled: bool, label: &str, duration: Duration, detail: &str) {
    if !enabled {
        return;
    }
    let millis = duration.as_secs_f64() * 1000.0;
    if detail.is_empty() {
        eprintln!("atlas timings: {label:<12} {millis:>8.2} ms");
    } else {
        eprintln!("atlas timings: {label:<12} {millis:>8.2} ms  {detail}");
    }
}

fn write_output_atomic(path: &Path, contents: &str) -> Result<(), i32> {
    if path.is_dir() {
        eprintln!("atlas: output path is a directory: {}", path.display());
        return Err(2);
    }

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        eprintln!(
            "atlas: output directory does not exist: {}",
            parent.display()
        );
        return Err(2);
    }
    if !parent.is_dir() {
        eprintln!(
            "atlas: output parent is not a directory: {}",
            parent.display()
        );
        return Err(2);
    }

    match write_output_atomic_inner(parent, path, contents) {
        Ok(()) => Ok(()),
        Err((tmp, err)) => {
            let _ = std::fs::remove_file(tmp);
            eprintln!("atlas: could not write {}: {err}", path.display());
            Err(1)
        }
    }
}

fn write_output_atomic_inner(
    parent: &Path,
    path: &Path,
    contents: &str,
) -> Result<(), (PathBuf, std::io::Error)> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("atlas-map");

    let mut last_err: Option<(PathBuf, std::io::Error)> = None;
    for attempt in 0..100 {
        let tmp = parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(mut file) => {
                if let Err(err) = file.write_all(contents.as_bytes()) {
                    return Err((tmp, err));
                }
                if let Err(err) = file.sync_all() {
                    return Err((tmp, err));
                }
                drop(file);
                replace_file(&tmp, path).map_err(|err| (tmp, err))?;
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                last_err = Some((tmp, err));
                continue;
            }
            Err(err) => return Err((tmp, err)),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        (
            parent.join(format!(".{file_name}.tmp-{}", std::process::id())),
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not allocate a temporary output path",
            ),
        )
    }))
}

fn replace_file(tmp: &Path, path: &Path) -> std::io::Result<()> {
    match std::fs::rename(tmp, path) {
        Ok(()) => Ok(()),
        Err(_err) if cfg!(windows) && path.exists() => {
            std::fs::remove_file(path)?;
            std::fs::rename(tmp, path)
        }
        Err(err) => Err(err),
    }
}

fn unsupported_extension_hint(root: &Path) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut scanned = 0usize;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            if file_type.is_dir() {
                if EXTENSION_HINT_SKIP_DIRS.contains(&name) {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() {
                scanned += 1;
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_ascii_lowercase();
                    *counts.entry(ext).or_default() += 1;
                }
                if scanned >= MAX_EXTENSION_HINT_FILES {
                    break;
                }
            }
        }
        if scanned >= MAX_EXTENSION_HINT_FILES {
            break;
        }
    }

    if counts.is_empty() {
        return None;
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|(a_ext, a_count), (b_ext, b_count)| {
        b_count.cmp(a_count).then_with(|| a_ext.cmp(b_ext))
    });
    let hint = ranked
        .into_iter()
        .take(5)
        .map(|(ext, count)| format!(".{ext} ({count})"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(hint)
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
                     {SUPPORTED_EXTENSION_SUMMARY} ({SUPPORTED_LANGUAGE_SUMMARY})"
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
        assert!(cli.output.is_none());
        assert!(cli.focus.is_empty());
        assert!(!cli.no_private);
        assert!(!cli.for_agent);
        assert!(!cli.timings);
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
            "--for-agent",
            "--timings",
            "--output",
            "atlas-map.md",
        ]);
        assert_eq!(cli.path, PathBuf::from("../somewhere"));
        assert_eq!(cli.budget, 4096);
        assert_eq!(cli.lang.as_deref(), Some("py,rs"));
        assert_eq!(cli.focus, vec!["src/auth", "src/api/routes.ts"]);
        assert!(cli.no_private);
        assert!(cli.for_agent);
        assert!(cli.timings);
        assert_eq!(cli.output, Some(PathBuf::from("atlas-map.md")));
    }

    #[test]
    fn short_flags_work() {
        let cli = parse(&[
            "atlas", "-b", "512", "-f", "json", "-l", "rs", "-o", "map.json",
        ]);
        assert_eq!(cli.budget, 512);
        assert_eq!(cli.format, Format::Json);
        assert_eq!(cli.lang.as_deref(), Some("rs"));
        assert_eq!(cli.output, Some(PathBuf::from("map.json")));
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
        assert_eq!(
            parse_langs(Some("go,java,c,cpp")).unwrap(),
            vec![Language::Go, Language::Java, Language::C, Language::Cpp]
        );
        assert_eq!(parse_langs(None).unwrap(), Vec::<Language>::new());
        assert_eq!(parse_langs(Some("cobol")).err(), Some(2));
    }

    #[test]
    fn agent_preamble_wraps_markdown() {
        let wrapped = with_agent_preamble("# atlas: demo\n");
        assert!(wrapped.starts_with("> atlas agent note:"));
        assert!(wrapped.ends_with("# atlas: demo\n"));
    }

    #[test]
    fn atomic_output_write_replaces_existing_file() {
        let dir = std::env::temp_dir().join(format!("atlas-cli-output-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("map.md");
        std::fs::write(&out, "old").unwrap();

        write_output_atomic(&out, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "new");
        let leftovers = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = std::fs::remove_dir_all(&dir);
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
