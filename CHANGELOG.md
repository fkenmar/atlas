# Changelog

All notable user-facing changes to repomap, grouped Added/Changed/Fixed per
release ([Keep a Changelog](https://keepachangelog.com/) style). Pre-1.0
semver policy: JSON schema changes bump the minor version — see the
release-process skill. Ranking/budgeting entries must include their
benchmark delta.

## [Unreleased]

### Changed
- **Adaptive skeleton footer** — the biggest density win so far. A
  full-granularity collapsed-file footer dominated the budget on large repos
  (pytest: 58 directory groups, ~32% of the whole map). The footer now coarsens
  to the *finest* directory depth whose group count stays ≤16, keeping the
  high-level shape while reclaiming the rest. pytest's footer fell from 2,491 →
  115 chars, and that freed ~30% of the budget now shows **10 files / 143
  symbol lines instead of 4** at the same 2,048-token budget — far more real API
  per token.
- **Lossless signature/import compression**: trailing syntactic noise carried
  in from the source line — an opening `{`, a Python/trait `:`/`;` — and import
  boilerplate (`use `, `;`, quotes) are dropped, keeping the declaration and
  dependency path intact. Same information, fewer tokens: repomap's own Rust
  self-map fell from 1,443 → 1,257 tokens (−13%) at budget 2,048, and a
  budget-filling map spends the freed space surfacing more real API. A pure
  step toward the token-reduction goal.
- Budget rung 3 gained a **partial-file rung**: when a file's full block won't
  fit, it now shows its top-K highest-PageRank symbols (default 8) plus a
  "… (N more symbols)" note, instead of collapsing straight to a one-line count.
  A too-large core file now surfaces its most important API rather than just
  its existence — more information at the same token budget (pytest's
  `src/_pytest/_py/path.py` went from "(101 symbols)" to its top 8 signatures +
  93 more). `omitted` added to BudgetedFile and the JSON schema.
- Symbol visibility is now language-aware instead of Python-underscore-only:
  Rust items are public iff declared `pub`/`pub(crate)` (bare fns, non-exported
  helpers, and non-`#[macro_export]` macros are correctly private); TypeScript
  members are private under a `private`/`protected` modifier. This makes the
  ladder's drop-private rung and `--no-private` actually work for Rust/TS
  (previously every symbol read as public). Known limitation: Rust trait
  methods carry no `pub` keyword so they read as private — documented, only
  bites under a tight budget. (No effect on the pytest benchmark — Python
  visibility is unchanged.)
- Budget rung 3 + ranking, both found by dogfooding the map on pytest (92k LOC):
  - **One-line rung:** a file whose full block overflows the remaining budget
    now collapses to a one-line summary (`## path (#rank, N symbols)`) instead
    of dropping the whole file — fixing a degenerate case where a single huge
    top-ranked file blanked out the entire map (pytest rendered 0 content files
    before this; now it shows the core modules).
  - **Ranking count-bias fix:** a file's score summed its symbols' raw PageRank,
    so a file's symbol COUNT dominated — 200-test-function files swamped the
    core API. Now each symbol contributes only its rank *earned above the
    uniform teleport baseline*, so trivial (never-referenced) symbols add ~0.
    pytest's top-ranked files flipped from `testing/test_*.py` to the
    `src/_pytest/*` core modules.
- Extraction now drops Rust inline test scaffolding: symbols (and their
  spurious import/call graph edges) inside `#[cfg(test)]` / `mod tests`
  modules are suppressed via tree-sitter node navigation — test fns, helpers,
  and the `mod tests` symbol itself are no longer mapped. They are noise, not
  API surface. Dogfood impact on repomap's own source: the map went from a
  degraded 2,036-token "params elided" listing with 6 of 16 files collapsed to
  a **1,749-token full-detail listing of all 16 files** — removing test noise
  freed enough budget to show the entire real API at full signature fidelity.

### Added
- M1 ignore handling (FR-7): discover now reads root-level `.gitignore` and
  `.repomapignore` and skips matching files/dirs during the walk (in addition
  to the built-in vendored-path defaults). The hand-rolled matcher supports the
  common forms — comments, blank lines, `dir/` (directory-only), `*` segment
  globs, basename patterns (match any path component), and root-relative path
  patterns. Not handled in v1: negation (`!`), `**`, nested ignore files
  (documented; a later milestone may adopt the `ignore` crate). No new
  dependency.
- M1 incremental cache (FR-6): parse results are cached in `.repomap/cache`
  (bincode), keyed on each file's content hash plus a cache version. Unchanged
  files reuse their stored parse instead of re-running tree-sitter; a changed
  hash or version bump invalidates. Files not seen in a run are pruned on save.
  The cache is purely an optimization — every I/O or decode error degrades
  silently to a cold parse, never an error — and `.repomap/` is gitignored.
  `parse_all` keeps its uncached signature (delegates to a disabled cache);
  the CLI uses the cached path. Cold and warm runs produce byte-identical maps.
- M1 JSON renderer (FR-5/json): `--format json` emits the versioned PRD §7.3
  schema (`version`, `repo`, `budget{target,rendered,detail}`, `files[{path,
  lang,rank,score,imported_by,one_line,symbols[{kind,name,sig,line,
  visibility}],imports}]`, `collapsed`, skip/unwired counts). Hand-serialized
  with spec-correct string escaping (no serde dependency yet) and deterministic
  output. `BudgetedFile`/`RenderedSymbol` gained `lang` and per-symbol `line`.
- M1 CLI integration: clap (derive) drives the full pipeline end-to-end —
  `repomap [PATH] --budget N --focus PATH... --lang csv --no-private`. discover
  → parse → link → rank (with `--focus` paths mapped to PageRank seeds) →
  budget → Markdown. repomap now compiles its own 2,473-LOC source into a
  2,036-token budgeted map. `--format` accepts `md` (json/xml are later rungs).
- M1 budget stage (FR-3, FR-11) + budgeted Markdown renderer (FR-5, md): greedy
  packing into a token budget (default 2,048) with exact BPE counts from
  tiktoken-rs `cl100k_base`, behind a pluggable `Tokenizer` trait. Degradation
  ladder per PRD §5.1, in order: drop private symbols → strip parameter names
  (bracket-depth-aware, types kept) → collapse low-rank files into a
  directory-skeleton footer that always retains every file (none lost).
  Detail reduction is global and tried first; only an overflowing most-compact
  listing triggers greedy per-file collapse. Deterministic (score-desc via
  `f64::total_cmp`, BTreeMap-grouped footer). A rust-reviewer pass caught and
  fixed a lost-file bug (empty files vanished from both listing and footer) and
  a param-stripping bug (operator `>` in a default masked the next param), both
  now regression-tested. Benchmark owed at M1 CLI integration (the budgeted
  path isn't wired into the CLI yet).
- M1 rank stage (FR-4): personalized PageRank over the link graph — in-house
  power iteration (damping 0.85, ≤20 iterations, L1 convergence check per
  PRD §7.2). Dangling Symbol-node mass is redistributed through the teleport
  vector each step so scores stay a distribution summing to 1; `--focus` node
  indices seed the personalization vector. Deterministic (node-index-order
  arithmetic). Property tests cover the distribution invariant, that
  referenced symbols outrank unreferenced ones, focus boosting, and
  determinism.
- M1 link stage (ADR-0002): best-effort syntactic import/reference graph —
  File nodes (index-aligned to the file list) plus Symbol nodes, with import
  edges (File→File) and reference edges (File→callee Symbol), sorted and
  deduplicated for determinism. Per-language import resolution (Python dotted,
  TypeScript relative, Rust `use` incl. `pub use`/aliases), all fuzzy by
  design (no type checker). A rust-reviewer pass caught and fixed four
  false-edge classes before commit: Python bare-name shadowing, Rust `pub use`
  mis-parsing, TypeScript `../` root-escape, and Rust deep-segment basename
  guessing — each now has a regression test (17 link tests total). Feeds
  PageRank; the benchmark runs at the M1 integration checkpoint (no budgeted
  map to inject until budgeting lands).
- M1 grammars (FR-1): TypeScript/JavaScript (tree-sitter-typescript 0.23.2)
  and Rust (tree-sitter-rust 0.24.2) wired end-to-end — per-language
  `grammar()` handle + compiled tags.scm query, with extraction snapshot tests
  now covering all three Tier 1 languages. Snapshot review caught two real
  extraction bugs: Rust trait methods with a default body were mis-tagged as
  free functions, and TypeScript ambient (`declare`) / overload signatures
  (`function_signature` nodes, distinct from `function_declaration`) were
  dropped entirely — both are now extracted.
- M0 naive pipeline: tree-sitter + Python grammar wired (tree-sitter 0.26.9,
  tree-sitter-python 0.25.0); discover walks with vendored-path/hidden-dir
  exclusion and sorted output; parse extracts defs/imports/calls via the
  embedded Python tags.scm; naive full-map markdown renderer with FR-12
  skip/unwired footer; minimal `repomap [PATH] [--lang csv]` CLI.
- Real `dump-ast` example (named-AST printer for wired grammars).
- Python extraction snapshot test (`UPDATE_SNAPSHOTS=1` to regenerate) and a
  discover-walk fixture test. Snapshot immediately caught and fixed a real
  bug: decorated methods extracted as functions.
- Benchmark harness executes real headless `claude -p` sessions per task in
  fresh pinned clones (acceptEdits mode), records tokens/turns/cost medians
  with per-run values and automatic >15%-spread variance notes, and
  `--record-baseline` writes baseline.json — the only sanctioned writer.
- No-map baseline recorded (M0 exit criterion): pytest 8.2.0,
  claude-sonnet-4-6, 3 runs/task — 902,555 tok / 22 turns (task 01),
  369,461 tok / 14 turns (task 02), 6/6 success-criteria passes.
- Preliminary with-map probe (unofficial; naive ~81k-token map injected):
  turns −41–43% on both tasks, 6/6 passes, but tokens/cost up 2.2–3.4×
  from per-turn cache re-reads of the oversized map — direct evidence that
  M1 budgeting is the load-bearing feature. Details in STATUS.md.

- Self-improvement loop: docs/SELF_IMPROVEMENT.md (pick → implement → gate →
  measure → keep-or-revert by stats), runnable as `/improve` (one iteration)
  or `/loop /improve` (continuous, self-paced); benchmark/history.md is the
  append-only stats ledger, seeded with the baseline and probe rows.
- Competitive-arms protocol (post-M1) in benchmark/README.md: repomap must
  beat Aider repo-map, ctags, and a file-tree control at equal budget.
- Comprehension benchmark (benchmark/comprehension.sh + verified question
  set for pytest 8.2.0): read-only Q&A sessions scored against answer keys;
  hard gate in the loop — with-map accuracy must be ≥ without-map, so the
  map can never trade correctness for token savings.

### Changed
- run.sh delivers prompts via stdin (naive maps exceed ARG_MAX as an
  argument) and keeps harness artifacts outside the working clone.
- Project skeleton: six-stage pipeline module stubs, embedded tags.scm
  queries for Python/TypeScript/Rust, benchmark harness scaffold, and the
  Claude Code workspace (hooks, subagents, skills, slash commands).
