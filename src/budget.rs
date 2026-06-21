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

/// How many top-ranked symbols a file shows in the partial rung before the
/// rest are summarized as "… (N more)". A starting point, benchmark-tunable.
const PARTIAL_SYMBOLS: usize = 8;

/// Per-file caps on index entries (top-N by symbol PageRank). Types get a
/// generous cap (a file like `capture.py` legitimately exports many classes);
/// functions get a small one. Both bound the breadth-vs-depth trade so a few
/// symbol-heavy top files can't crowd out lower-ranked files. Benchmark-tunable.
const INDEX_TYPES_PER_FILE: usize = 8;
const INDEX_FUNCS_PER_FILE: usize = 2;

/// Fraction of the budget reserved for the compact symbol index when files
/// overflow into the footer (rung 3). Trades some full-signature detail on the
/// marginal files for name→file coverage of the whole long tail — the lever the
/// comprehension benchmark rewards (answer-in-map ⇒ one-turn answer). Benchmark-
/// tunable; 0.0 disables the index and restores the pre-index packing.
const INDEX_RESERVE_FRACTION: f64 = 0.40;

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
    /// Whether `--no-private` was user-requested (vs. the budget forcing
    /// private symbols out) — lets the renderer word the header honestly.
    pub requested_no_private: bool,
    /// Included files, in rank order.
    pub files: Vec<BudgetedFile>,
    /// Low-rank files dropped to the footer, grouped by directory and sorted.
    pub collapsed: Vec<CollapsedDir>,
    /// Compact name→file index of navigable symbols whose defining file didn't
    /// fit in full (collapsed, one-line, or partial files). Ordered by file
    /// rank; greedily truncated to the budget. Empty when the full listing fit.
    pub symbol_index: Vec<IndexedSymbol>,
    pub skipped_files: usize,
    pub unwired_files: usize,
}

pub struct BudgetedFile {
    pub rel: String,
    /// Lowercase language name (`python`/`typescript`/`rust`), for JSON.
    pub lang: &'static str,
    /// 1-based display rank.
    pub rank: usize,
    pub score: f64,
    /// Import-edge in-degree — “imported by N files”.
    pub imported_by: usize,
    /// Resolved + display imports (raw import strings, deduped, sorted).
    pub imports: Vec<String>,
    /// Reverse dependencies — files that import this one ("used by"), capped.
    /// The edit sites for a change to this file's API.
    pub used_by: Vec<String>,
    pub symbols: Vec<RenderedSymbol>,
    /// Ladder rung 3 (per-file): the full block didn't fit, so render only a
    /// one-line summary (`## path (#rank, N symbols)`) — keeps a too-large
    /// top-ranked file from blanking out the whole map. `symbols` is retained
    /// for the count.
    pub one_line: bool,
    /// Partial rung: count of symbols dropped to fit (`symbols` then holds the
    /// top-ranked survivors, displayed in source order). 0 = the file is whole.
    pub omitted: usize,
}

pub struct RenderedSymbol {
    pub kind: SymbolKind,
    pub name: String,
    /// Signature after any detail degradation (param-name stripping).
    pub signature: String,
    pub visibility: Visibility,
    /// 1-based source line of the declaration (for JSON).
    pub line: usize,
}

pub struct CollapsedDir {
    /// Directory key, e.g. `src/utils` (or `.` for the repo root).
    pub dir: String,
    pub count: usize,
}

/// One entry in the compact symbol index: a navigable declaration (class,
/// interface, enum, type, top-level function, or constant — not methods or
/// fields) from a file whose full signatures didn't fit the budget, mapped to
/// the file that defines it. The index trades a full signature (~20 tokens)
/// for a bare name→path (~6), so an agent can answer "where is `X` defined?"
/// from the map instead of grepping — at a fraction of the cost of showing the
/// file in full. Built only when the budget forces files into the footer.
#[derive(Clone)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    /// Repo-relative path of the file that declares it.
    pub rel: String,
    /// Stable ADR 0009 anchor: `relpath#name`, or `relpath#name@line` only when
    /// the same file declares the same name more than once.
    pub anchor: String,
    /// 1-based source line of the declaration name.
    pub line: usize,
}

/// A named type — class, interface, enum, or type alias. These are what an
/// agent navigates *to* ("which class implements X?"), so the index lists them
/// first and most generously: when a file can't be shown in full, knowing its
/// types is worth more than knowing its helper functions.
fn is_type_like(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum | SymbolKind::TypeAlias
    )
}

/// A top-level function or constant — indexed in a second tier, after every
/// file's types, with whatever budget remains. Methods/fields are reached via
/// their class; `module` is a whole-file marker — none earn an index line.
fn is_func_like(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Function | SymbolKind::Constant)
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
    // Per-file PageRank-derived score, per-symbol scores (for the partial
    // rung's top-K selection), and import in-degree.
    let scores = file_scores(files, graph, ranking);
    let sym_scores = per_symbol_scores(files, graph, ranking);
    let imported_by = file_import_indegree(files.len(), graph);
    let imports = resolved_imports(files, graph);
    let used_by = reverse_imports(files, graph);

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
        requested_no_private: opts.no_private,
        files: Vec::new(),
        collapsed: Vec::new(),
        symbol_index: Vec::new(),
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
            let f = build_file(
                fi,
                files,
                &scores,
                &sym_scores[fi],
                &imported_by,
                &imports[fi],
                &used_by[fi],
                rank + 1,
                detail,
                None,
            );
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
    // is tried most-informative-first — full block, then a top-K partial (its
    // highest-ranked symbols), then a one-line summary — so a too-large file
    // surfaces as much of its real API as fits instead of collapsing whole
    // (the pytest failure mode). Anything that doesn't fit even as one line
    // falls to the directory-skeleton footer (none lost). Re-render on each
    // candidate so the count stays exact.
    //
    // We reserve a slice of the budget for the symbol index (below): packing
    // files only up to `file_ceiling` leaves room to list the *names* of the
    // collapsed tail, which is far cheaper per symbol than a full block and is
    // what lets an agent locate the long tail without grepping.
    let detail = Detail::NoParams;
    let reserve = (opts.budget_tokens as f64 * INDEX_RESERVE_FRACTION) as usize;
    let file_ceiling = opts.budget_tokens.saturating_sub(reserve);
    let mut shown = vec![false; files.len()];
    let mut included: Vec<BudgetedFile> = Vec::new();
    for (rank, &fi) in order.iter().enumerate() {
        let full = build_file(
            fi,
            files,
            &scores,
            &sym_scores[fi],
            &imported_by,
            &imports[fi],
            &used_by[fi],
            rank + 1,
            detail,
            None,
        );
        if full.symbols.is_empty() && full.imports.is_empty() {
            continue; // empty → collapsed via the complement below
        }
        let partial = build_file(
            fi,
            files,
            &scores,
            &sym_scores[fi],
            &imported_by,
            &imports[fi],
            &used_by[fi],
            rank + 1,
            detail,
            Some(PARTIAL_SYMBOLS),
        );
        let mut one_line = clone_file(&full);
        one_line.one_line = true;
        // Try in order; the partial only differs from full when it dropped
        // symbols, so skip it otherwise.
        let candidates = [
            Some(full),
            (partial.omitted > 0).then_some(partial),
            Some(one_line),
        ];
        let mut placed = false;
        for candidate in candidates.into_iter().flatten() {
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
            if measure(&map, counter) <= file_ceiling {
                shown[fi] = true;
                included.push(candidate);
                placed = true;
                break;
            }
        }
        if !placed {
            // Even a one-line entry overflows → the file budget is full; this
            // file and every lower-ranked one fall to the footer (and are
            // candidates for the symbol index below).
            break;
        }
    }
    let candidates = build_symbol_index(&order, files, &shown, &included, &sym_scores);
    let mut map = BudgetedMap {
        detail,
        collapsed: collapse_complement(&order, &shown, files),
        files: included,
        ..clone_base(&base)
    };
    // Fill the reserved slice with as many index entries (highest-ranked first)
    // as fit the full budget.
    fit_symbol_index(&mut map, candidates, opts.budget_tokens, counter);
    let tokens = measure(&map, counter);
    finalize(map, tokens)
}

/// Build the candidate symbol index from navigable declarations whose defining
/// file isn't shown in full: every symbol of a collapsed or one-line file, and
/// the omitted symbols of a partial file. Files appear in rank order (via
/// `order`); within each file only its [`INDEX_SYMBOLS_PER_FILE`] highest-scored
/// symbols are kept (then restored to source order), so a few symbol-heavy
/// top-ranked files can't monopolize the index and starve lower-ranked files of
/// coverage — the breadth is what lets the index reach a rank-43 core file.
/// Private symbols are excluded, matching the compact detail level (rung 3
/// already drops them from the file blocks).
fn build_symbol_index(
    order: &[usize],
    files: &[(SourceFile, ParsedFile)],
    shown: &[bool],
    included: &[BudgetedFile],
    sym_scores: &[Vec<f64>],
) -> Vec<IndexedSymbol> {
    // Two tiers, concatenated: every file's types (in file-rank order) first,
    // then every file's functions. The budget cut (binary search downstream)
    // therefore fills types across the whole rank order before spending a token
    // on functions — so a rank-43 core class still lands while rank-2's helper
    // functions wait.
    let mut types = Vec::new();
    let mut funcs = Vec::new();
    for &fi in order {
        let (src, parsed) = &files[fi];
        let duplicate_names = duplicate_symbol_names(parsed);
        // Names already rendered in full above, to avoid duplicating them. A
        // one-line file shows no symbols, so none of its names are "shown".
        let shown_here: Vec<&str> = if shown[fi] {
            match included.iter().find(|f| f.rel == src.rel) {
                Some(f) if !f.one_line => f.symbols.iter().map(|s| s.name.as_str()).collect(),
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        let pick = |keep: fn(SymbolKind) -> bool, cap: usize, sink: &mut Vec<IndexedSymbol>| {
            let mut cand: Vec<usize> = parsed
                .symbols
                .iter()
                .enumerate()
                .filter(|(_, s)| keep(s.kind) && s.visibility == Visibility::Public)
                .filter(|(_, s)| !shown_here.contains(&s.name.as_str()))
                .map(|(i, _)| i)
                .collect();
            if cand.len() > cap {
                // Keep the highest-scored symbols, then restore source order so
                // the rendered line reads top-to-bottom.
                cand.sort_by(|&a, &b| {
                    score_at(&sym_scores[fi], b).total_cmp(&score_at(&sym_scores[fi], a))
                });
                cand.truncate(cap);
                cand.sort_unstable();
            }
            for i in cand {
                let s = &parsed.symbols[i];
                sink.push(IndexedSymbol {
                    name: s.name.clone(),
                    kind: s.kind,
                    rel: src.rel.clone(),
                    anchor: symbol_anchor(&src.rel, &s.name, s.line, &duplicate_names),
                    line: s.line,
                });
            }
        };
        pick(is_type_like, INDEX_TYPES_PER_FILE, &mut types);
        pick(is_func_like, INDEX_FUNCS_PER_FILE, &mut funcs);
    }
    types.extend(funcs);
    types
}

pub(crate) fn duplicate_symbol_names(parsed: &ParsedFile) -> Vec<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for symbol in &parsed.symbols {
        *counts.entry(symbol.name.as_str()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(name, _)| name.to_string())
        .collect()
}

pub(crate) fn symbol_anchor(
    rel: &str,
    name: &str,
    line: usize,
    duplicate_names: &[String],
) -> String {
    if duplicate_names.iter().any(|n| n == name) {
        format!("{rel}#{name}@{line}")
    } else {
        format!("{rel}#{name}")
    }
}

/// Greedily include the longest rank-ordered prefix of `candidates` whose
/// rendered map stays within `budget`. Token count is monotonic in the prefix
/// length (entries only add lines), so a binary search finds the cut in
/// O(log n) measurements instead of re-rendering per entry.
fn fit_symbol_index<T: Tokenizer>(
    map: &mut BudgetedMap,
    candidates: Vec<IndexedSymbol>,
    budget: usize,
    counter: &T,
) {
    if candidates.is_empty() {
        return;
    }
    let (mut lo, mut hi) = (0usize, candidates.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        map.symbol_index = candidates[..mid].to_vec();
        if measure(map, counter) <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    map.symbol_index = candidates[..lo].to_vec();
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

/// PageRank score of each symbol, indexed `[file][symbol-within-file]`, for
/// the partial rung's top-K selection. Symbols not represented as graph nodes
/// keep a 0.0 score.
fn per_symbol_scores(
    files: &[(SourceFile, ParsedFile)],
    graph: &Graph,
    ranking: &Ranking,
) -> Vec<Vec<f64>> {
    let mut out: Vec<Vec<f64>> = files
        .iter()
        .map(|(_, parsed)| vec![0.0f64; parsed.symbols.len()])
        .collect();
    for (idx, node) in graph.nodes.iter().enumerate() {
        if node.kind == NodeKind::Symbol {
            if let Some(si) = node.symbol {
                if let Some(slot) = out.get_mut(node.file).and_then(|f| f.get_mut(si)) {
                    *slot = ranking.score(idx);
                }
            }
        }
    }
    out
}

/// Resolved internal imports per file: the repo-relative paths of the files
/// each file actually depends on (File→File edges from the link graph). This
/// replaces the raw import strings in the map, which were dominated by
/// stdlib/external noise (`std::collections`, `node:path`, …) that costs tokens
/// without aiding navigation — the in-repo dependency structure is the signal.
fn resolved_imports(files: &[(SourceFile, ParsedFile)], graph: &Graph) -> Vec<Vec<String>> {
    let num_files = files.len();
    let mut out: Vec<Vec<String>> = vec![Vec::new(); num_files];
    for (fi, adj) in graph.edges.iter().enumerate().take(num_files) {
        for &target in adj {
            if let Some(node) = graph.nodes.get(target) {
                if node.kind == NodeKind::File {
                    out[fi].push(node.label.clone());
                }
            }
        }
    }
    for deps in &mut out {
        deps.sort();
        deps.dedup();
    }
    out
}

/// How many top-ranked "used by" (reverse-dependency) files to list per file
/// before truncating; the header's `imported_by` count still gives the total.
const USED_BY_CAP: usize = 8;

/// Reverse-dependency edges: for each file, the repo-relative paths of the
/// files that import it ("used by"). To change a symbol you must visit
/// everything that uses it, so this is the signal a multi-site edit needs and
/// the benchmark flagged as missing. Mirrors [`resolved_imports`] with the
/// edge reversed; sorted + deduped.
fn reverse_imports(files: &[(SourceFile, ParsedFile)], graph: &Graph) -> Vec<Vec<String>> {
    let num_files = files.len();
    let mut out: Vec<Vec<String>> = vec![Vec::new(); num_files];
    for (importer, adj) in graph.edges.iter().enumerate().take(num_files) {
        for &target in adj {
            if let Some(node) = graph.nodes.get(target) {
                if node.kind == NodeKind::File {
                    out[node.file].push(files[importer].0.rel.clone());
                }
            }
        }
    }
    for deps in &mut out {
        deps.sort();
        deps.dedup();
        deps.truncate(USED_BY_CAP);
    }
    out
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

#[allow(clippy::too_many_arguments)]
fn build_file(
    fi: usize,
    files: &[(SourceFile, ParsedFile)],
    scores: &[f64],
    symbol_scores: &[f64],
    imported_by: &[usize],
    resolved_imports: &[String],
    used_by: &[String],
    rank: usize,
    detail: Detail,
    max_symbols: Option<usize>,
) -> BudgetedFile {
    let (src, parsed) = &files[fi];
    // Keep the original index alongside each visible symbol so we can rank by
    // PageRank score for the partial rung yet still display in source order.
    let mut visible: Vec<usize> = parsed
        .symbols
        .iter()
        .enumerate()
        .filter(|(_, s)| detail.includes_private() || s.visibility == Visibility::Public)
        .map(|(i, _)| i)
        .collect();
    let omitted = match max_symbols {
        Some(k) if visible.len() > k => {
            // Keep the k highest-scoring symbols, then restore source order.
            visible.sort_by(|&a, &b| {
                score_at(symbol_scores, b).total_cmp(&score_at(symbol_scores, a))
            });
            let dropped = visible.len() - k;
            visible.truncate(k);
            visible.sort_unstable();
            dropped
        }
        _ => 0,
    };
    let symbols = visible
        .iter()
        .map(|&i| {
            let s = &parsed.symbols[i];
            let sig = if detail.strips_params() {
                strip_param_names(&s.signature)
            } else {
                s.signature.clone()
            };
            RenderedSymbol {
                kind: s.kind,
                name: s.name.clone(),
                signature: tidy_signature(&sig),
                visibility: s.visibility,
                line: s.line,
            }
        })
        .collect();
    BudgetedFile {
        rel: src.rel.clone(),
        lang: src.lang.name(),
        rank,
        score: scores[fi],
        imported_by: imported_by[fi],
        imports: resolved_imports.to_vec(),
        used_by: used_by.to_vec(),
        symbols,
        one_line: false,
        omitted,
    }
}

fn score_at(scores: &[f64], i: usize) -> f64 {
    scores.get(i).copied().unwrap_or(0.0)
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
/// Group collapsed files into the skeleton footer. On a large repo a
/// full-granularity listing dominates the budget (pytest: 58 groups, ~32% of
/// the map), so coarsen to the *finest* directory depth whose group count
/// stays within [`MAX_FOOTER_GROUPS`] — keeping the high-level shape while
/// freeing the bulk of those tokens for real API.
fn collapse_dirs(indices: &[usize], files: &[(SourceFile, ParsedFile)]) -> Vec<CollapsedDir> {
    const MAX_FOOTER_GROUPS: usize = 16;
    let max_depth = indices
        .iter()
        .map(|&fi| dir_depth(&files[fi].0.rel))
        .max()
        .unwrap_or(1)
        .max(1);
    // Group count is monotonic non-decreasing in depth; keep the finest depth
    // that still fits, falling back to the coarsest (depth 1).
    let mut chosen = grouped_dirs(indices, files, 1);
    for depth in 2..=max_depth {
        let groups = grouped_dirs(indices, files, depth);
        if groups.len() <= MAX_FOOTER_GROUPS {
            chosen = groups;
        } else {
            break;
        }
    }
    chosen
}

/// Number of directory segments in a relative path (`a/b/c.py` → 2).
fn dir_depth(rel: &str) -> usize {
    rel.matches('/').count()
}

/// Group collapsed files by the first `depth` segments of their directory.
fn grouped_dirs(
    indices: &[usize],
    files: &[(SourceFile, ParsedFile)],
    depth: usize,
) -> Vec<CollapsedDir> {
    let mut by_dir: BTreeMap<String, usize> = BTreeMap::new();
    for &fi in indices {
        let rel = &files[fi].0.rel;
        let dir = rel.rsplit_once('/').map_or("", |(d, _)| d);
        // A root-level file (no directory) still belongs in the skeleton.
        let key = if dir.is_empty() {
            ".".to_string()
        } else {
            dir.split('/')
                .take(depth.max(1))
                .collect::<Vec<_>>()
                .join("/")
        };
        *by_dir.entry(key).or_insert(0) += 1;
    }
    by_dir
        .into_iter()
        .map(|(dir, count)| CollapsedDir { dir, count })
        .collect()
}

/// Drop trailing syntactic noise carried in from the source line — an opening
/// brace, or a Python/trait trailing `:` / `;` — that costs tokens without
/// adding information. Lossless: `class Service {` → `class Service`,
/// `def run() -> None:` → `def run() -> None`, `fn ready(&self);` →
/// `fn ready(&self)`.
fn tidy_signature(sig: &str) -> String {
    sig.trim_end()
        .trim_end_matches(['{', ':', ';'])
        .trim_end()
        .to_string()
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
        requested_no_private: base.requested_no_private,
        files: Vec::new(),
        collapsed: Vec::new(),
        symbol_index: Vec::new(),
        skipped_files: base.skipped_files,
        unwired_files: base.unwired_files,
    }
}

fn clone_file(f: &BudgetedFile) -> BudgetedFile {
    BudgetedFile {
        rel: f.rel.clone(),
        lang: f.lang,
        rank: f.rank,
        score: f.score,
        imported_by: f.imported_by,
        imports: f.imports.clone(),
        used_by: f.used_by.clone(),
        symbols: f
            .symbols
            .iter()
            .map(|s| RenderedSymbol {
                kind: s.kind,
                name: s.name.clone(),
                signature: s.signature.clone(),
                visibility: s.visibility,
                line: s.line,
            })
            .collect(),
        one_line: f.one_line,
        omitted: f.omitted,
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
        pfile_imports(rel, symbols, &[])
    }

    fn pfile_imports(
        rel: &str,
        symbols: Vec<Symbol>,
        imports: &[&str],
    ) -> (SourceFile, ParsedFile) {
        (
            SourceFile {
                path: std::path::PathBuf::from(rel),
                rel: rel.to_string(),
                lang: crate::lang::Language::Python,
            },
            ParsedFile {
                symbols,
                imports: imports.iter().map(|s| s.to_string()).collect(),
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
    fn tidy_signature_is_lossless() {
        // Trailing syntactic noise dropped, the declaration kept intact.
        assert_eq!(tidy_signature("class Service {"), "class Service");
        assert_eq!(
            tidy_signature("def run(self) -> None:"),
            "def run(self) -> None"
        );
        assert_eq!(tidy_signature("fn ready(&self);"), "fn ready(&self)");
        assert_eq!(
            tidy_signature("pub const X: u32 = 5;"),
            "pub const X: u32 = 5"
        );
    }

    #[test]
    fn imports_show_resolved_internal_deps_only() {
        // a.py imports b.py (in-repo) and `os` (stdlib). The map's import line
        // lists only the resolved in-repo dependency, not the stdlib noise.
        let files = vec![
            pfile_imports(
                "a.py",
                vec![sym("f", Visibility::Public, "def f()")],
                &["b", "os"],
            ),
            pfile("b.py", vec![sym("g", Visibility::Public, "def g()")]),
        ];
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 10_000,
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(2), &opts, &WordCounter);
        let a = map
            .files
            .iter()
            .find(|f| f.rel == "a.py")
            .expect("a.py shown");
        assert_eq!(
            a.imports,
            vec!["b.py".to_string()],
            "only the in-repo dep, no stdlib"
        );
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
        // The huge top file must degrade — a partial (top-K) view or, failing
        // that, a one-line summary — never blank the map.
        assert!(
            map.files.iter().any(|f| f.one_line || f.omitted > 0),
            "the huge top file should degrade to partial or one-line"
        );
        let collapsed: usize = map.collapsed.iter().map(|c| c.count).sum();
        assert_eq!(map.files.len() + collapsed, 6, "none lost");
    }

    #[test]
    fn partial_rung_keeps_highest_scored_symbols() {
        // A file with one widely-referenced symbol + many unreferenced ones,
        // under a budget too tight for the full block, must keep the
        // referenced symbol (highest PageRank) in its partial view.
        let mut syms = vec![sym("hot", Visibility::Public, "def hot() -> int")];
        for i in 0..40 {
            syms.push(sym(
                &format!("cold{i}"),
                Visibility::Public,
                &format!("def cold{i}(a: int, b: int) -> int"),
            ));
        }
        let mut files = vec![pfile("big.py", syms)];
        // Several callers reference `hot`, lifting its symbol PageRank.
        for i in 0..4 {
            files.push(pfile(
                &format!("caller{i}.py"),
                vec![sym("c", Visibility::Public, "def c() -> int")],
            ));
            // make caller reference `hot`
            files[i + 1].1.references.push("hot".to_string());
        }
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            // Wide enough for the top-K partial block but not all 41 symbols.
            // Headroom accounts for the rung-3 index reserve, which lowers the
            // file ceiling below the nominal budget.
            budget_tokens: 200,
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(5), &opts, &WordCounter);
        let big = map
            .files
            .iter()
            .find(|f| f.rel == "big.py")
            .expect("big.py shown");
        assert!(
            big.omitted > 0,
            "big.py should be partial under a tight budget"
        );
        assert!(
            big.symbols.iter().any(|s| s.name == "hot"),
            "the widely-referenced symbol must survive the top-K cut"
        );
    }

    fn cls(name: &str, sig: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            kind: SymbolKind::Class,
            signature: sig.to_string(),
            line: 1,
            visibility: Visibility::Public,
        }
    }

    #[test]
    fn symbol_index_surfaces_collapsed_classes() {
        // A tight budget collapses the lower-ranked files. Their public classes
        // must still be locatable via the symbol index (name → path), even
        // though their full blocks didn't fit. Types are listed ahead of any
        // helper functions.
        let mut files = vec![pfile(
            "core.py",
            vec![sym("run", Visibility::Public, "def run() -> int")],
        )];
        for i in 0..12 {
            files.push(pfile(
                &format!("src/mod{i:02}.py"),
                vec![
                    cls(&format!("Widget{i}"), &format!("class Widget{i}")),
                    sym(
                        &format!("helper{i}"),
                        Visibility::Public,
                        &format!("def helper{i}() -> int"),
                    ),
                ],
            ));
        }
        let g = graph_of(&files);
        let r = crate::rank::rank(&g, &[]);
        let opts = BudgetOptions {
            budget_tokens: 90, // tight: most files collapse into the footer
            no_private: false,
        };
        let map = pack(&files, &g, &r, "demo", stats(13), &opts, &WordCounter);
        // Some files collapsed (so the index has something to do) ...
        assert!(map.files.len() < 13, "expected some files collapsed");
        assert!(!map.symbol_index.is_empty(), "index should carry the tail");
        // ... and a collapsed file's class is locatable by name → path.
        let entry = map
            .symbol_index
            .iter()
            .find(|e| e.name.starts_with("Widget"))
            .expect("a collapsed Widget class should be indexed");
        assert!(entry.rel.starts_with("src/mod"));
        assert_eq!(entry.kind, SymbolKind::Class);
        assert_eq!(entry.anchor, format!("{}#{}", entry.rel, entry.name));
        assert!(entry.line >= 1);
        // Types come before functions: the first function (if any) appears only
        // after every indexed type.
        let first_fn = map
            .symbol_index
            .iter()
            .position(|e| e.kind == SymbolKind::Function);
        let last_type = map
            .symbol_index
            .iter()
            .rposition(|e| e.kind == SymbolKind::Class);
        if let (Some(f), Some(t)) = (first_fn, last_type) {
            assert!(f > t, "all types should precede any function in the index");
        }
    }

    #[test]
    fn symbol_anchor_adds_line_only_for_duplicate_names() {
        let parsed = ParsedFile {
            symbols: vec![
                Symbol {
                    kind: SymbolKind::Function,
                    name: "from".to_string(),
                    signature: "fn from()".to_string(),
                    visibility: Visibility::Public,
                    line: 10,
                },
                Symbol {
                    kind: SymbolKind::Function,
                    name: "from".to_string(),
                    signature: "fn from(value: u8)".to_string(),
                    visibility: Visibility::Public,
                    line: 20,
                },
                Symbol {
                    kind: SymbolKind::Class,
                    name: "Widget".to_string(),
                    signature: "class Widget".to_string(),
                    visibility: Visibility::Public,
                    line: 30,
                },
            ],
            imports: Vec::new(),
            references: Vec::new(),
            lines: 3,
        };
        let duplicates = duplicate_symbol_names(&parsed);
        assert_eq!(
            symbol_anchor("src/x.rs", "from", 10, &duplicates),
            "src/x.rs#from@10"
        );
        assert_eq!(
            symbol_anchor("src/x.rs", "Widget", 30, &duplicates),
            "src/x.rs#Widget"
        );
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
