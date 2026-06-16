//! Stage 5 — budget: greedily pack ranked symbols into the token budget
//! using exact BPE token counts (tiktoken-rs, FR-11). Degradation ladder
//! (PRD §5.1), applied in order until the map fits:
//!   1. drop private symbols,
//!   2. drop parameter names (keep types),
//!   3. collapse low-rank files into a directory-grouped footer (the repo's
//!      directory skeleton is always retained).
//!
//! Detail reduction (rungs 1–2) is global and tried first; only if the most
//! compact full listing still overflows do we greedily include files in rank
//! order and collapse the tail (rung 3). Exact token counts come from
//! rendering the candidate map (see `crate::render::markdown`) and counting —
//! so the budgeted number is the number actually emitted, not an estimate.
//!
//! Determinism (NFR-4): files are ordered by (score desc, path asc) via
//! `f64::total_cmp`; symbols keep source-line order; collapsed groups are
//! sorted by directory. No reliance on HashMap iteration order.

use std::collections::BTreeMap;

use crate::discover::SourceFile;
use crate::link::{Graph, NodeKind};
use crate::parse::{ParseStats, ParsedFile, SymbolKind, Visibility};
use crate::rank::Ranking;

/// Default token budget (PRD G1 / §5.2).
pub const DEFAULT_BUDGET: usize = 2048;

/// Exact-count tokenizer (FR-11: pluggable). Implementors count the BPE
/// tokens in a rendered map; the budget packer is generic over this so tests
/// can use a cheap stand-in instead of loading a real vocabulary.
pub trait Tokenizer {
    fn count(&self, text: &str) -> usize;
}

/// Exact BPE counter backed by tiktoken-rs `cl100k_base` (FR-11). Claude's
/// tokenizer isn't published as a crate; cl100k_base is the standard proxy for
/// budget enforcement (a future flag can swap the encoding). Loading the
/// vocabulary is comparatively expensive, so build one and reuse it.
pub struct TiktokenCounter {
    bpe: tiktoken_rs::CoreBPE,
}

impl TiktokenCounter {
    pub fn cl100k() -> Result<Self, String> {
        let bpe = tiktoken_rs::cl100k_base().map_err(|e| e.to_string())?;
        Ok(TiktokenCounter { bpe })
    }
}

impl Tokenizer for TiktokenCounter {
    fn count(&self, text: &str) -> usize {
        self.bpe.encode_ordinary(text).len()
    }
}

/// Global signature-detail level — the first two ladder rungs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    /// Every symbol, full signatures.
    Full,
    /// Private symbols dropped.
    NoPrivate,
    /// Private dropped *and* parameter names stripped (types kept).
    NoParams,
}

impl Detail {
    fn includes_private(self) -> bool {
        matches!(self, Detail::Full)
    }
    fn strips_params(self) -> bool {
        matches!(self, Detail::NoParams)
    }
}

/// Options that steer packing.
pub struct BudgetOptions {
    pub budget_tokens: usize,
    /// `--no-private`: public surface only, regardless of budget headroom.
    pub no_private: bool,
}

impl Default for BudgetOptions {
    fn default() -> Self {
        BudgetOptions {
            budget_tokens: DEFAULT_BUDGET,
            no_private: false,
        }
    }
}

/// What survived packing: the selection and per-file detail the renderers
/// consume. Carries enough display metadata (repo name, LOC, ranks, footer)
/// that a renderer needs no other input.
pub struct BudgetedMap {
    pub repo_name: String,
    pub target_tokens: usize,
    /// Exact token count of the rendered map (set by `pack`).
    pub rendered_tokens: usize,
    pub total_loc: usize,
    pub total_files: usize,
    /// Global detail level applied (ladder rungs 1–2).
    pub detail: Detail,
    /// Included files, in rank order.
    pub files: Vec<BudgetedFile>,
    /// Low-rank files dropped to the footer, grouped by directory and sorted.
    pub collapsed: Vec<CollapsedDir>,
    pub skipped_files: usize,
    pub unwired_files: usize,
}

pub struct BudgetedFile {
    pub rel: String,
    /// 1-based display rank.
    pub rank: usize,
    pub score: f64,
    /// Import-edge in-degree — “imported by N files”.
    pub imported_by: usize,
    /// Resolved + display imports (raw import strings, deduped, sorted).
    pub imports: Vec<String>,
    pub symbols: Vec<RenderedSymbol>,
    /// Ladder rung 3 (per-file): the full block didn't fit, so render only a
    /// one-line summary (`## path (#rank, N symbols)`) — keeps a too-large
    /// top-ranked file from blanking out the whole map. `symbols` is retained
    /// for the count.
    pub one_line: bool,
}

pub struct RenderedSymbol {
    pub kind: SymbolKind,
    pub name: String,
    /// Signature after any detail degradation (param-name stripping).
    pub signature: String,
    pub visibility: Visibility,
}

pub struct CollapsedDir {
    /// Directory key, e.g. `src/utils` (or `.` for the repo root).
    pub dir: String,
    pub count: usize,
}

/// Pack ranked symbols into `opts.budget_tokens`, applying the degradation
/// ladder deterministically. `repo_name`/`stats` supply the header figures.
pub fn pack<T: Tokenizer>(
    files: &[(SourceFile, ParsedFile)],
    graph: &Graph,
    ranking: &Ranking,
    repo_name: &str,
    stats: ParseStats,
    opts: &BudgetOptions,
    counter: &T,
) -> BudgetedMap {
    // Per-file PageRank-derived score and import in-degree.
    let scores = file_scores(files, graph, ranking);
    let imported_by = file_import_indegree(files.len(), graph);

    // Files in rank order (score desc, path asc — deterministic).
    let mut order: Vec<usize> = (0..files.len()).collect();
    order.sort_by(|&a, &b| {
        scores[b]
            .total_cmp(&scores[a])
            .then_with(|| files[a].0.rel.cmp(&files[b].0.rel))
    });

    let base = BudgetedMap {
        repo_name: repo_name.to_string(),
        target_tokens: opts.budget_tokens,
        rendered_tokens: 0,
        total_loc: stats.total_lines,
        total_files: stats.parsed_files,
        detail: Detail::Full,
        files: Vec::new(),
        collapsed: Vec::new(),
        skipped_files: stats.skipped_files,
        unwired_files: stats.unwired_files,
    };

    // Ladder rungs 1–2: try ever-more-compact *complete* listings.
    let levels: &[Detail] = if opts.no_private {
        &[Detail::NoPrivate, Detail::NoParams]
    } else {
        &[Detail::Full, Detail::NoPrivate, Detail::NoParams]
    };
    for &detail in levels {
        let mut shown = vec![false; files.len()];
        let mut included: Vec<BudgetedFile> = Vec::new();
        for (rank, &fi) in order.iter().enumerate() {
            let f = build_file(fi, files, &scores, &imported_by, rank + 1, detail);
            if f.symbols.is_empty() && f.imports.is_empty() {
                continue; // no visible content → falls to the skeleton footer
            }
            shown[fi] = true;
            included.push(f);
        }
        let map = BudgetedMap {
            detail,
            files: included,
            collapsed: collapse_complement(&order, &shown, files),
            ..clone_base(&base)
        };
        let tokens = measure(&map, counter);
        if tokens <= opts.budget_tokens {
            return finalize(map, tokens);
        }
    }

    // Rung 3: at the most compact detail, greedily include by rank. Each file
    // is tried at its full block, then — if that overflows — as a one-line
    // summary, so a single huge top-ranked file can't blank out the whole map
    // (the pytest failure mode). Anything that doesn't fit even as one line
    // collapses into the directory-skeleton footer (none lost). Re-render on
    // each candidate so the count stays exact.
    let detail = Detail::NoParams;
    let mut shown = vec![false; files.len()];
    let mut included: Vec<BudgetedFile> = Vec::new();
    for (rank, &fi) in order.iter().enumerate() {
        let file = build_file(fi, files, &scores, &imported_by, rank + 1, detail);
        if file.symbols.is_empty() && file.imports.is_empty() {
            continue; // empty → collapsed via the complement below
        }
        let mut placed = false;
        for one_line in [false, true] {
            let mut candidate = clone_file(&file);
            candidate.one_line = one_line;
            let mut trial_shown = shown.clone();
            trial_shown[fi] = true;
            let mut trial = clone_files(&included);
            trial.push(clone_file(&candidate));
            let map = BudgetedMap {
                detail,
                files: trial,
                collapsed: collapse_complement(&order, &trial_shown, files),
                ..clone_base(&base)
            };
            if measure(&map, counter) <= opts.budget_tokens {
                shown[fi] = true;
                included.push(candidate);
                placed = true;
                break;
            }
        }
        if !placed {
            // Even a ~one-line entry overflows → the budget is full; this file
            // and every lower-ranked one fall to the footer.
            break;
        }
    }
    let map = BudgetedMap {
        detail,
        files: included,
        collapsed: collapse_complement(&order, &shown, files),
        ..clone_base(&base)
    };
    let tokens = measure(&map, counter);
    finalize(map, tokens)
}

/// Render the map and count its tokens. The header shows `rendered_tokens`,
/// which we don't know until after counting — a ~1-token self-reference we
/// accept rather than iterate to a fixed point (the budget is a target, not a
/// hard cap, per PRD §5.2 / the §5.3 example showing 1,991 under 2,048).
fn measure<T: Tokenizer>(map: &BudgetedMap, counter: &T) -> usize {
    counter.count(&crate::render::markdown::render(map))
}

fn finalize(mut map: BudgetedMap, tokens: usize) -> BudgetedMap {
    map.rendered_tokens = tokens;
    map
}

/// File score = its File-node PageRank (import importance) + the sum of its
/// symbols' *earned* rank — each symbol's score above the uniform teleport
/// baseline. Subtracting the baseline is what keeps a file from ranking high
/// on symbol COUNT alone: a 200-test-function file whose symbols are never
/// referenced contributes ~0 and stays where its (low) import rank puts it,
/// while a file with a few widely-referenced symbols rises. Without this, raw
/// symbol count dominated and test files swamped the core API.
fn file_scores(files: &[(SourceFile, ParsedFile)], graph: &Graph, ranking: &Ranking) -> Vec<f64> {
    let n = graph.nodes.len();
    let baseline = if n > 0 { 1.0 / n as f64 } else { 0.0 };
    let mut scores = vec![0.0f64; files.len()];
    for (i, score) in scores.iter_mut().enumerate() {
        *score = ranking.score(i); // File node i
    }
    for (idx, node) in graph.nodes.iter().enumerate() {
        if node.kind == NodeKind::Symbol {
            if let Some(s) = scores.get_mut(node.file) {
                *s += (ranking.score(idx) - baseline).max(0.0);
            }
        }
    }
    scores
}

/// Import in-degree of each File node (“imported by N files”).
fn file_import_indegree(num_files: usize, graph: &Graph) -> Vec<usize> {
    let mut indeg = vec![0usize; num_files];
    for adj in &graph.edges {
        for &target in adj {
            if let Some(node) = graph.nodes.get(target) {
                if node.kind == NodeKind::File {
                    indeg[node.file] += 1;
                }
            }
        }
    }
    indeg
}

fn build_file(
    fi: usize,
    files: &[(SourceFile, ParsedFile)],
    scores: &[f64],
    imported_by: &[usize],
    rank: usize,
    detail: Detail,
) -> BudgetedFile {
    let (src, parsed) = &files[fi];
    let symbols = parsed
        .symbols
        .iter()
        .filter(|s| detail.includes_private() || s.visibility == Visibility::Public)
        .map(|s| RenderedSymbol {
            kind: s.kind,
            name: s.name.clone(),
            signature: if detail.strips_params() {
                strip_param_names(&s.signature)
            } else {
                s.signature.clone()
            },
            visibility: s.visibility,
        })
        .collect();
    BudgetedFile {
        rel: src.rel.clone(),
        rank,
        score: scores[fi],
        imported_by: imported_by[fi],
        imports: parsed.imports.clone(),
        symbols,
        one_line: false,
    }
}

/// Collapse every file NOT in `shown` into the directory-skeleton footer, so
/// no file is ever lost — including files with no extractable symbols (PRD
/// §5.1: the directory skeleton is always retained).
fn collapse_complement(
    order: &[usize],
    shown: &[bool],
    files: &[(SourceFile, ParsedFile)],
) -> Vec<CollapsedDir> {
    let hidden: Vec<usize> = order.iter().copied().filter(|&fi| !shown[fi]).collect();
    collapse_dirs(&hidden, files)
}

/// Group a set of file indices by directory for the collapsed footer.
fn collapse_dirs(indices: &[usize], files: &[(SourceFile, ParsedFile)]) -> Vec<CollapsedDir> {
    let mut by_dir: BTreeMap<String, usize> = BTreeMap::new();
    for &fi in indices {
        let rel = &files[fi].0.rel;
        // A file with no visible content still belongs in the skeleton.
        let dir = rel.rsplit_once('/').map_or(".", |(d, _)| d).to_string();
        *by_dir.entry(dir).or_insert(0) += 1;
    }
    by_dir
        .into_iter()
        .map(|(dir, count)| CollapsedDir { dir, count })
        .collect()
}

/// Strip parameter *names* from a signature, keeping the types (ladder rung
/// 2). Bracket-depth aware so generics/tuples with commas survive. Best
/// effort: a parameter with no `:` annotation is kept verbatim (e.g. `self`).
fn strip_param_names(sig: &str) -> String {
    let Some(open) = sig.find('(') else {
        return sig.to_string();
    };
    let mut depth = 0i32;
    let mut close = None;
    for (i, c) in sig[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return sig.to_string();
    };
    let params = &sig[open + 1..close];
    if params.trim().is_empty() {
        return sig.to_string();
    }
    let stripped: Vec<String> = split_top_level(params)
        .into_iter()
        .map(strip_one_param)
        .collect();
    format!(
        "{}({}){}",
        &sig[..open],
        stripped.join(", "),
        &sig[close + 1..]
    )
}

/// Split a parameter list on top-level commas, respecting `() [] {} <>` depth.
fn split_top_level(params: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in params.char_indices() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            // `>`/`)`/… also appear as operators in default values; clamp at 0
            // so a stray closer can't drive depth negative and mask a comma.
            ')' | ']' | '}' | '>' => depth = (depth - 1).max(0),
            ',' if depth == 0 => {
                out.push(&params[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&params[start..]);
    out
}

/// `name: Type` → `Type`; `name` (no annotation) → kept as-is.
fn strip_one_param(param: &str) -> String {
    let p = param.trim();
    // Find the first top-level `:` (skip `::` and bracketed colons).
    let mut depth = 0i32;
    let bytes = p.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth = (depth - 1).max(0),
            b':' if depth == 0 => {
                if bytes.get(i + 1) == Some(&b':') {
                    i += 2; // skip a `::` path separator
                    continue;
                }
                return p[i + 1..].trim().to_string();
            }
            _ => {}
        }
        i += 1;
    }
    p.to_string()
}

// `..clone_base` helpers: BudgetedMap isn't Clone (it holds Vecs of non-Clone
// display structs), so we hand-roll shallow rebuilds for the struct-update
// candidates above.
fn clone_base(base: &BudgetedMap) -> BudgetedMap {
    BudgetedMap {
        repo_name: base.repo_name.clone(),
        target_tokens: base.target_tokens,
        rendered_tokens: 0,
        total_loc: base.total_loc,
        total_files: base.total_files,
        detail: base.detail,
        files: Vec::new(),
        collapsed: Vec::new(),
        skipped_files: base.skipped_files,
        unwired_files: base.unwired_files,
    }
}

fn clone_file(f: &BudgetedFile) -> BudgetedFile {
    BudgetedFile {
        rel: f.rel.clone(),
        rank: f.rank,
        score: f.score,
        imported_by: f.imported_by,
        imports: f.imports.clone(),
        symbols: f
            .symbols
            .iter()
            .map(|s| RenderedSymbol {
                kind: s.kind,
                name: s.name.clone(),
                signature: s.signature.clone(),
                visibility: s.visibility,
            })
            .collect(),
        one_line: f.one_line,
    }
}

fn clone_files(fs: &[BudgetedFile]) -> Vec<BudgetedFile> {
    fs.iter().map(clone_file).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Symbol;

    /// Cheap stand-in for tiktoken: one "token" per whitespace-delimited word.
    /// Monotonic in length, which is all the ladder logic needs.
    struct WordCounter;
    impl Tokenizer for WordCounter {
        fn count(&self, text: &str) -> usize {
            text.split_whitespace().count()
        }
    }

    fn sym(name: &str, vis: Visibility, sig: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: sig.to_string(),
            line: 1,
            visibility: vis,
        }
    }

    fn pfile(rel: &str, symbols: Vec<Symbol>) -> (SourceFile, ParsedFile) {
        (
            SourceFile {
                path: std::path::PathBuf::from(rel),
                rel: rel.to_string(),
                lang: crate::lang::Language::Python,
            },
            ParsedFile {
                symbols,
                imports: Vec::new(),
                references: Vec::new(),
                lines: 10,
            },
        )
    }

    fn stats(n: usize) -> ParseStats {
        ParseStats {
            parsed_files: n,
            skipped_files: 0,
            unwired_files: 0,
            total_lines: 100,
        }
    }

    fn graph_of(files: &[(SourceFile, ParsedFile)]) -> Graph {
        crate::link::link(files)
    }

    #[test]
    fn tiktoken_counter_is_exact() {
        let c = TiktokenCounter::cl100k().expect("cl100k_base vocabulary loads");
        // cl100k_base: "hello" + " world" = 2 tokens.
        assert_eq!(c.count("hello world"), 2);
        assert!(c.count("def authenticate(email: str) -> Session") > 0);
    }

    #[test]
    fn strip_param_names_keeps_types() {
        assert_eq!(
            strip_param_names("def authenticate(email: str, password: str) -> Session"),
            "def authenticate(str, str) -> Session"
        );
        // Generics with internal commas survive.
        assert_eq!(
            strip_param_names("fn insert(key: Map<String, u32>, value: u32)"),
            "fn insert(Map<String, u32>, u32)"
        );
        // `self` and unannotated params are kept verbatim.
        assert_eq!(
            strip_param_names("def run(self) -> None"),
            "def run(self) -> None"
        );
        // No params: unchanged.
        assert_eq!(strip_param_names("def now() -> int"), "def now() -> int");
        // No parens at all: unchanged.
        assert_eq!(
            strip_param_names("const API_VERSION: str = ..."),
            "const API_VERSION: str = ..."
        );
        // A comparison operator in a default must not mask the param comma —
        // the second param's name is still stripped.
        assert_eq!(
            strip_param_names("def f(a=1 > 0, b: str) -> None"),
            "def f(a=1 > 0, str) -> None"
        );
    }

    #[test]
    fn empty_files_land_in_the_skeleton_not_lost() {
        // Three content files + two empty ones; generous budget so the full
        // listing fits. The empty files must still appear in the collapsed
        // footer — shown + collapsed must equal the total (none lost).
        let mut files = vec![
            pfile("a.py", vec![sym("f", Visibility::Public, "def f() -> int")]),
            pfile("b.py", vec![sym("g", Visibility::Public, "def g() -> int")]),
            pfile(
                "src/c.py",
                vec![sym("h", Visibility::Public, "def h() -> int")],
            ),
        ];
        files.push(pfile("src/empty1.py", vec![])); // no symbols, no imports
        files.push(pfile("src/empty2.py", vec![]));
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 10_000,
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(5), &opts, &WordCounter);
        let collapsed: usize = map.collapsed.iter().map(|c| c.count).sum();
        assert_eq!(map.files.len(), 3);
        assert_eq!(
            map.files.len() + collapsed,
            5,
            "every file must be shown or in the skeleton footer (none lost)"
        );
    }

    #[test]
    fn everything_fits_keeps_full_detail() {
        let files = vec![
            pfile(
                "a.py",
                vec![sym("f", Visibility::Public, "def f(x: int) -> int")],
            ),
            pfile(
                "b.py",
                vec![sym("g", Visibility::Public, "def g() -> None")],
            ),
        ];
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 10_000,
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(2), &opts, &WordCounter);
        assert_eq!(map.detail, Detail::Full);
        assert_eq!(map.files.len(), 2);
        assert!(map.collapsed.is_empty());
        assert!(map.rendered_tokens <= opts.budget_tokens);
    }

    #[test]
    fn tight_budget_collapses_low_rank_files() {
        // Many files, tiny budget → only the top few survive, rest collapse.
        let files: Vec<_> = (0..20)
            .map(|i| {
                pfile(
                    &format!("src/mod{i:02}.py"),
                    vec![sym("f", Visibility::Public, "def f(a: int, b: int) -> int")],
                )
            })
            .collect();
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 40, // very tight
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(20), &opts, &WordCounter);
        assert!(map.files.len() < 20, "expected some files collapsed");
        let collapsed_count: usize = map.collapsed.iter().map(|c| c.count).sum();
        assert_eq!(
            map.files.len() + collapsed_count,
            20,
            "every file is either shown or in the skeleton footer (none lost)"
        );
    }

    #[test]
    fn huge_top_file_becomes_one_line_not_empty_map() {
        // Regression for the pytest failure mode: one enormous top-ranked file
        // whose full block exceeds the budget must collapse to a one-line
        // summary, NOT blank out the map — the smaller files still show.
        let big: Vec<Symbol> = (0..60)
            .map(|i| {
                sym(
                    &format!("big_fn_{i}"),
                    Visibility::Public,
                    &format!("def big_fn_{i}(a: int, b: int, c: int) -> int"),
                )
            })
            .collect();
        let mut files = vec![pfile("huge.py", big)];
        for i in 0..5 {
            files.push(pfile(
                &format!("small{i}.py"),
                vec![sym("s", Visibility::Public, "def s() -> int")],
            ));
        }
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 80, // too small for huge.py's full block
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(6), &opts, &WordCounter);
        assert!(
            !map.files.is_empty(),
            "map must not be empty when smaller files fit"
        );
        assert!(
            map.files.iter().any(|f| f.one_line),
            "the huge top file should collapse to a one-line summary"
        );
        let collapsed: usize = map.collapsed.iter().map(|c| c.count).sum();
        assert_eq!(map.files.len() + collapsed, 6, "none lost");
    }

    #[test]
    fn no_private_flag_drops_private_symbols() {
        let files = vec![pfile(
            "a.py",
            vec![
                sym("public_fn", Visibility::Public, "def public_fn() -> int"),
                sym("_hidden", Visibility::Private, "def _hidden() -> int"),
            ],
        )];
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 10_000,
            no_private: true,
        };
        let map = pack(&files, &g, &r, "demo", stats(1), &opts, &WordCounter);
        let names: Vec<&str> = map.files[0]
            .symbols
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(names, vec!["public_fn"]);
        assert_ne!(map.detail, Detail::Full);
    }

    #[test]
    fn is_deterministic() {
        let files: Vec<_> = (0..8)
            .map(|i| {
                pfile(
                    &format!("src/f{i}.py"),
                    vec![sym(
                        "run",
                        Visibility::Public,
                        "def run(a: int, b: str) -> bool",
                    )],
                )
            })
            .collect();
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 60,
            no_private: false,
        };
        let a = pack(&files, &g, &r, "demo", stats(8), &opts, &WordCounter);
        let b = pack(&files, &g, &r, "demo", stats(8), &opts, &WordCounter);
        assert_eq!(a.rendered_tokens, b.rendered_tokens);
        let names = |m: &BudgetedMap| m.files.iter().map(|f| f.rel.clone()).collect::<Vec<_>>();
        assert_eq!(names(&a), names(&b));
    }
}
