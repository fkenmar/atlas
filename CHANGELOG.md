# Changelog

All notable user-facing changes to atlas, grouped Added/Changed/Fixed per
release ([Keep a Changelog](https://keepachangelog.com/) style). Pre-1.0
semver policy: JSON schema changes bump the minor version — see the
release-process skill. Ranking/budgeting entries must include their
benchmark delta.

## [Unreleased]

### Changed
- **Parallel parsing (rayon, #4).** The parse stage now runs the expensive
  tree-sitter parse of cache-miss files across cores via rayon, while cache
  access and output assembly stay sequential so the map remains deterministic
  (NFR-4). Cold-path speedup on large repos; warm (fully cached) runs are
  unaffected. Verified cold==warm output and run-to-run determinism.

### Added
- **MCP server (`atlas serve --mcp`, #7).** atlas now runs as an MCP server over
  stdio (newline-delimited JSON-RPC 2.0) so an LLM agent can pull a fresh map as
  a tool call instead of being handed a static one. One tool, `get_map(path,
  budget?, no_private?, format?)`, returns the rendered map (md/json/xml).
  Hand-dispatched with `serde_json` (no framework); the protocol core is a pure,
  unit-tested `handle()` function. See ADR 0008.
- **Experimental agent-output CLI polish (`dev`).** `atlas map` mode now has
  `-o/--output` for same-directory temp-file + rename writes, `--for-agent` for
  a short Markdown-only agent preamble, `--timings` for stage timings on stderr,
  and empty-map diagnostics that report the top file extensions atlas saw. Normal
  stdout output is unchanged unless a new flag is used.
- **Experimental MCP stdio server (`atlas serve --mcp`, #7).** Adds a
  hand-dispatched JSON-RPC/MCP path with `initialize`, `tools/list`,
  `tools/call`, `ping`, and one read-only `get_map` tool that returns Markdown,
  JSON, or XML map content. Uses `serde_json` only; see ADR 0008.
- **Tier 2 language grammars: Go, Java, C, C++ (#10).** atlas now maps Go
  (`.go`), Java (`.java`), and C/C++ (`.c`/`.h`/`.cpp`/`.hpp`/…) alongside the
  Tier 1 set (Python, TypeScript/JavaScript, Rust) — functions/methods, types
  (struct/class/interface/enum), fields, constants, and import/include edges,
  each with a tags.scm validated by a committed extraction snapshot. Visibility
  is per-language (Go capitalization, Java/TS `private`/`protected`, C/C++
  `static`). Go import resolution matches in-repo packages by longest path
  suffix; Java FQNs and C/C++ `#include`s resolve like dotted/relative paths.
  Fixes a constant-name filter that wrongly dropped non-`UPPER_SNAKE` constants
  for languages with an explicit `const` keyword (Go's PascalCase consts now
  appear; the filter still applies to Python/TS assignment-style constants).
- **`atlas diff <old> <new>`.** A structural delta between two trees: added/removed
  files, and per common file the added/removed symbols, **changed signatures**, and
  added/removed import edges — so an agent sees what moved at the signature/edge level
  without re-reading the tree. Runs on the raw parse output (no ranking/budget, so
  every change is reported), with deterministic Markdown, JSON, and XML output
  (`--format`, #16) — the structured forms reuse the map renderers' escapers.
  Each side is a directory **or a git revision** — `atlas diff HEAD~1 HEAD`,
  `atlas diff v0.2.0 .` — revisions checked out via the `git` CLI into a temp
  worktree (no `git2` dependency; ADR 0007, #18). No new dependency. See ADR 0005.
  Closes #12.
  - A declaration that keeps its name but changes kind (e.g. a free `fn` moved
    into an `impl`, function → method) is paired into a single `±` change entry
    instead of an unexplained remove + add (ADR 0006, #17). Conservative: only
    unique-name 1↔1 matches pair; overloads stay as add/remove.
- **XML renderer (`--format xml`).** A third output format alongside Markdown
  and JSON, for prompt-injection-safe wrapping in Claude prompts: a well-formed
  XML document where signatures/paths are escaped per the XML 1.0 spec so
  embedded source can't break out of the structure. It describes the *same*
  logical schema as the JSON renderer (shares the schema version and the
  kind/visibility/detail vocabulary), and output stays deterministic (NFR-4).
  Closes the last output-format item on the board (#11).

## [0.2.1-alpha] - 2026-06-17

Terminal polish, and the fix that makes the pip publish actually land.

### Fixed
- **PyPI macOS x86_64 wheel.** The publish pipeline stalled on v0.2.0-alpha: the
  `macos-13` runner that built the Intel-macOS wheel sat queued indefinitely
  (the runner is being deprecated), blocking the publish job. Both macOS wheels
  now build on the `macos-14` (arm64) runner — x86_64 via cross-compile (Apple's
  SDK is universal) — so `pip install --pre atlas-map` publishes reliably.

### Added
- **Colorized terminal output.** When stdout is a terminal, the Markdown map is
  ANSI-colorized for scannability — bold-cyan header, bold-blue file paths, dim
  rank/dependency/footer detail, tinted declaration keywords. It's a display-only
  layer: **piped output stays plain** (so feeding the map to an agent or a file
  is unaffected), and the token count/determinism are untouched. Controlled by
  `--color auto|always|never` (default `auto`); honors the `NO_COLOR` convention.
- **Shell completions.** `atlas --completions <bash|zsh|fish|powershell|elvish>`
  prints a completion script to stdout (e.g. `atlas --completions zsh >
  ~/.zfunc/_atlas`). Adds the `clap_complete` dependency.

## [0.2.0-alpha] - 2026-06-17

Second alpha. Headline: the **symbol index** more than doubles the comprehension
token win (−65.2% at 20/20 accuracy), and atlas now installs via **pip/pipx**
(`pip install --pre atlas-map`) alongside the existing `curl | sh` and `cargo
install`.

### Added
- **pip / pipx distribution (`pip install atlas-map`).** atlas now ships as a
  PyPI wheel so the Python-native audience (aider/Claude-Code/Cursor users) can
  install it without a Rust toolchain or a `curl | sh`, and reach it on Windows
  and in locked-down/corp environments where the shell installer doesn't work.
  The wheel wraps the same compiled binary via maturin `bindings = "bin"` (no
  Python runs at execution time; the `atlas` command lands on PATH); a new
  `.github/workflows/pypi.yml` builds the five platform wheels + an sdist
  fallback and publishes via PyPI Trusted Publishing (OIDC) on each release tag.
  Distribution name is `atlas-map` (bare `atlas` was taken); the command stays
  `atlas`. Additive — cargo-dist's `curl | sh`, archives, and `cargo install`
  are unchanged. *Maintainer one-time setup before first publish: add the PyPI
  Trusted Publisher (project `atlas-map`, repo `fkenmar/atlas`, workflow
  `pypi.yml`, environment `pypi`).*
- **Symbol index for the collapsed tail (ADR 0004).** When files overflow the
  budget, atlas now lists the *names* of the collapsed files' types — a compact
  `path: ClassA, ClassB` index — instead of erasing them into a bare directory
  skeleton, so an agent can locate the long tail without grepping. Type-first,
  ranked, capped per file; appears only when files collapse. The JSON output
  gains an additive `symbol_index` array (schema version unchanged). **Benchmark
  delta:** comprehension **−65.2% tokens** (85,670 → 29,781) at **20/20 accuracy
  in both arms**, median turns 3 → 1, at the default 2,048 budget — more than
  doubling the prior −30.1% (run-20260617-084740).
- **Prebuilt binary distribution via cargo-dist.** A GitHub Actions release
  workflow (`.github/workflows/release.yml`) cross-builds `atlas` for macOS,
  Linux, and Windows (x86_64 + arm64) and publishes a `curl | sh` shell
  installer plus per-platform archives on every version tag. dist config lives
  in `dist-workspace.toml`.

### Changed
- **Leaner published crate.** A Cargo.toml `exclude` drops dev-only process
  dirs (`.claude`, `.github`, `benchmark`, `docs`) and process docs from the
  packaged artifact: `cargo package` went from 81 to 41 files (55.8 KiB
  compressed) and still builds standalone.

## [0.1.0-alpha] - 2026-06-16

First tagged pre-release. The renamed **atlas** binary with the full M1 pipeline
(discover → parse → link → rank → budget → render) for Python / TypeScript-JS /
Rust. **Pre-release:** the M1 "measurable benchmark win" exit criterion is not yet
confirmed at N≥5 (tracked in #1); numbers below are from N=3 checkpoints.

### Changed
- **Imports show resolved internal dependencies, not raw import strings.** The
  import line was 27.5% of the map and dominated by stdlib/external noise
  (`std::collections`, `node:path`). It now lists only the repo-relative paths
  of the in-repo files each file actually depends on (the link graph's
  File→File edges) — e.g. `imports: parse.rs, lang/mod.rs`. On repomap's own
  source the import lines fell 1,090 → 283 chars (−74%), and the freed budget
  lifted the map from degraded "public-only" to full detail. The external/
  stdlib deps that drop out remain inferable from the symbol signatures; the
  in-repo dependency structure (the navigational signal) is now clearer.
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
- **Reverse-dependency ("used by") edges** — each file now lists the files
  that import it (`used by: a.rs, b.rs`, capped at 8; the header `imported_by`
  count gives the total). This is the signal a *multi-site edit* needs — to
  change a file's API you must visit everything that uses it — which the
  benchmark flagged as missing on multi-site tasks (the map's weakest case).
  In both the Markdown and JSON output. Whether it nets out (navigation value
  vs. added per-turn cost) is for the benchmark to judge on the next run.
- **Class fields are now extracted (Python)** — PRD §5.3 shows fields in the
  map (`class User # fields: id, email, …`), but classes were rendered without
  them. Annotated class-body attributes (dataclass/attrs fields, typed class
  attributes — `name: Type` / `name: Type = default`) are now extracted as a
  new `field` symbol kind and rendered indented under their class. This gives
  field-editing tasks (e.g. "add a field to a dataclass") the existing field
  set directly, without reading the file. Visibility follows the underscore
  convention; the cache version bumped (extraction output changed). Rust struct
  fields and TS class/interface members are a follow-up.
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
