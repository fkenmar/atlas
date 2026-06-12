# repomap — Product Requirements Document

A repo-to-context compiler: compress any codebase into a token-budgeted structural map for LLM coding agents.

| | |
|---|---|
| **Status** | Draft v0.1 (rev. 2 — Rust) |
| **Owner** | Kenmar |
| **Date** | June 12, 2026 |
| **Target release** | v0.1 alpha — 5 weeks from kickoff |
| **Language** | Rust (2021 edition, stable) |
| **License** | MIT (open source) |

## 1. Summary

repomap is a standalone CLI tool that parses an entire codebase and emits a compressed structural "map" of it: every function signature, type definition, exported symbol, and import relationship — with zero function bodies. A 50,000-line repository compiles down to a ~2,000-token summary that an LLM coding agent (Claude Code, Cursor, Codex CLI, or a raw API call) can hold entirely in context.

The closest existing implementation is Aider's repo-map, which is widely cited as one of the main reasons Aider performs well on large codebases — but it is embedded inside Aider's Python application, not consumable as a standalone artifact, library, or shell pipeline. repomap extracts that idea into a fast, deterministic, single-binary tool written in Rust, designed to be composed: piped into prompts, dropped into CLAUDE.md, exposed as an MCP server, or run in CI to keep a committed map fresh.

## 2. Problem

### 2.1 The context problem

LLM coding agents operate under a hard context budget. On a non-trivial repository, an agent cannot read everything, so it spends turns exploring: grepping, opening files, reading the wrong ones, and re-reading files it has already seen after compaction. This exploration is the dominant cost in both latency and tokens for agentic coding sessions, and it degrades output quality — an agent that doesn't know `AuthService.refresh_token()` exists will reimplement it.

### 2.2 Why current options fall short

| Option | What it does | Why it's insufficient |
|---|---|---|
| Aider repo-map | Tree-sitter-based map with graph ranking; the proven approach | Coupled to Aider's internals and Python runtime. No standalone CLI, no stable output contract, not usable from Claude Code or shell pipelines. |
| ctags / universal-ctags | Symbol index across many languages | Flat symbol list with no relationship graph, no ranking, no token budgeting, output format hostile to LLM consumption. |
| tree-sitter CLI | Raw parse trees per file | A primitive, not a product. No cross-file linking, ranking, or compression. |
| Manual CLAUDE.md / docs | Hand-written architecture notes | Goes stale immediately; never covers the full symbol surface; expensive to maintain. |
| Embedding-based RAG | Semantic retrieval of code chunks | Heavy infra, non-deterministic, retrieval misses structural relationships (who imports whom), and chunks still include bodies. |

### 2.3 Why now

Agentic coding tools went mainstream in 2025–2026, and context-engineering is now a recognized discipline. Every Claude Code power user is hand-rolling some version of this (tree of files, ctags dumps, pasted headers). The demand is validated by Aider's results; the standalone supply simply doesn't exist.

## 3. Goals and Non-Goals

### 3.1 Goals

- **G1 — Compression:** Reduce a repository's structural surface to a target token budget (default 2,048) with graceful degradation: highest-value symbols survive, low-value ones are pruned first.
- **G2 — Fidelity:** Every emitted signature is parser-accurate (tree-sitter grammars), never regex-scraped. Types, arity, visibility, and import edges are correct.
- **G3 — Speed:** Cold run on a 50k-LOC repo in under 2 seconds; warm (cached) run in under 200 ms. Fast enough to run on every agent turn or git hook.
- **G4 — Composability:** One static binary. Reads a directory, writes to stdout. Markdown, JSON, and XML output. Usable in pipes, CI, CLAUDE.md generation, and as an MCP server.
- **G5 — Determinism:** Same repo + same flags = byte-identical output. Diffs of committed maps are meaningful in code review.

### 3.2 Non-Goals (v1)

- Semantic search or embeddings — repomap is purely structural.
- Full code intelligence (go-to-definition, rename) — that's an LSP's job.
- IDE plugins or GUI — CLI and MCP only.
- Editing or generating code — read-only tool.
- Languages beyond the Tier 1/2 set (Section 6) — breadth comes after the ranking and budgeting core is right.

## 4. Target Users and Use Cases

| Persona | Use case | Success looks like |
|---|---|---|
| Claude Code power user (primary; includes the author) | Runs `repomap` in a session hook or pre-commit to keep a fresh map in CLAUDE.md / context | Agent stops re-exploring; fewer turns to first correct edit; lower token spend per task |
| Agent-framework developer | Calls repomap as a subprocess or MCP tool to seed agent context before planning | One dependency replaces a homegrown ctags/grep pipeline |
| OSS maintainer | Commits `REPOMAP.md` to the repo so any contributor's agent onboards instantly | Map regenerated in CI; PR diffs show API surface changes at a glance |
| Tech lead / reviewer | Uses the map diff as a lightweight API-surface change report | Breaking-change visibility without reading full diffs |

## 5. Product Description

### 5.1 Core pipeline

```
discover -> parse -> link -> rank -> budget -> render
(walk +    (tree-   (resolve (PageRank  (greedy   (md /
 ignore    sitter   imports,  over the   pack to   json /
 rules)    per      defs <->  symbol     N tokens) xml)
           file)    refs)     graph)
```

- **Discover:** Walk the tree respecting .gitignore, .repomapignore, and binary/vendored-path heuristics (node_modules, target/, .venv, dist/ excluded by default).
- **Parse:** Per-file tree-sitter parse extracting declarations: functions/methods (name, parameters, return type, visibility, async/generic markers), types (records, variants, classes, interfaces, enums, type aliases), constants, and module-level docstrings (first line only).
- **Link:** Build a directed graph: files and symbols as nodes; import statements and cross-file references as edges. Resolution is best-effort and syntactic (module paths, relative imports) — no type checker required.
- **Rank:** Personalized PageRank over the graph (the approach Aider validated). Symbols referenced from many places rank high; leaf utilities rank low. Optional personalization vector boosts files passed via `--focus` (e.g., files currently being edited), so the map adapts to the task.
- **Budget:** Greedy packing of ranked symbols into the token budget using exact token counts. Degradation order: drop private symbols → drop parameter names (keep types) → collapse files to one-line summaries → drop file entirely. The repo's directory skeleton is always retained.
- **Render:** Markdown (default, optimized for LLM readability), JSON (stable schema for programmatic consumers), XML (for prompt-injection-safe wrapping in Claude prompts).

### 5.2 CLI surface

```
$ repomap .                          # map cwd, 2048-token budget, markdown to stdout
$ repomap . --budget 4096 --format json
$ repomap . --focus src/auth/ --focus src/api/routes.ts
$ repomap . --lang ts,py             # restrict languages
$ repomap . --no-private             # public API surface only
$ repomap . --watch                  # daemon: re-emit on file change
$ repomap serve --mcp                # expose as MCP server (get_map, get_symbol tools)
$ repomap diff HEAD~5                # API-surface diff between git revisions
```

### 5.3 Example output (markdown, excerpt)

```
# repomap: acme-api (50,312 LOC, 214 files) | budget 2048 | rendered 1,991 tok

## src/auth/service.py (ranked #1 — imported by 31 files)
class AuthService:
    def authenticate(email: str, password: str) -> Session | None
    def refresh_token(token: str) -> Token            # raises TokenExpired
    def revoke_all(user_id: UUID) -> int

## src/api/routes.ts (#2)
export function registerRoutes(app: Express): void
export const API_VERSION: string
imports: ../auth/client, ./middleware/rateLimit

## src/db/models.py (#3)
class User(Base)      # fields: id, email, created_at, role
class Session(Base)   # fields: id, user_id, expires_at
...
[37 low-rank files collapsed: src/utils/* (14), tests/* (23)]
```

## 6. Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| FR-1 | Parse and extract declarations via tree-sitter for Tier 1 languages: TypeScript/JavaScript, Python, Rust. | P0 |
| FR-2 | Tier 2 languages: Go, Java, C/C++, OCaml. Added after the v0.1 core is stable. | P1 |
| FR-3 | Token-budgeted output with deterministic greedy packing and the degradation ladder in §5.1. | P0 |
| FR-4 | PageRank ranking over the import/reference graph with --focus personalization. | P0 |
| FR-5 | Markdown, JSON, and XML renderers with a versioned, documented JSON schema. | P0 (md/json), P1 (xml) |
| FR-6 | Incremental cache keyed on (file path, content hash, grammar version); only changed files re-parse. | P0 |
| FR-7 | Ignore handling: .gitignore, .repomapignore, built-in vendored-path defaults. | P0 |
| FR-8 | MCP server mode exposing get_map(budget, focus) and get_symbol(name) tools. | P1 |
| FR-9 | --watch daemon mode for editor/agent integrations. | P1 |
| FR-10 | repomap diff <rev>: API-surface diff between two git revisions. | P2 |
| FR-11 | Exact BPE token counting (tiktoken-rs) for budget enforcement; tokenizer selectable via flag. | P0 |
| FR-12 | Graceful handling of unparseable files: skip, count, report in a footer line — never crash. | P0 |

### Non-Functional Requirements

| ID | Requirement |
|---|---|
| NFR-1 | Performance: ≤2 s cold / ≤200 ms warm on a 50k-LOC repo (M-series laptop baseline); ≤30 s cold on 1M LOC. |
| NFR-2 | Distribution: single static binary per platform (macOS arm64/x64, Linux x64/arm64 musl); no runtime dependencies. Installable via Homebrew, cargo install, and curl-pipe script (all generated by cargo-dist). |
| NFR-3 | Memory: ≤500 MB peak on 1M-LOC repositories. |
| NFR-4 | Determinism: identical inputs and flags produce byte-identical output across runs and platforms. |
| NFR-5 | Privacy: fully offline; no network calls, no telemetry in v1. |

## 7. Technical Design Notes

### 7.1 Why Rust

Runtime performance is roughly language-neutral here — tree-sitter (a C library) does the heavy lifting regardless of host. Rust wins on development velocity and distribution: the tree-sitter project maintains first-party Rust bindings (the tree-sitter CLI itself is Rust), and most grammars ship as ready-made crates, so adding a language is a Cargo.toml line rather than a binding project. The crate ecosystem covers the entire feature list (CLI, parallelism, JSON, file watching, git, exact BPE tokenization), and cargo-dist solves cross-platform static-binary distribution in an afternoon. An OCaml implementation was evaluated and rejected: its main dependency (Semgrep's ocaml-tree-sitter) would require vendoring and binding grammar C sources by hand — a de-risking spike Rust simply doesn't need.

### 7.2 Key components

| Component | Choice | Notes |
|---|---|---|
| Parsing | tree-sitter crate (first-party) + per-language grammar crates, versions pinned in Cargo.lock | Pinned grammar crate versions give determinism (NFR-4) and stable cache keys (FR-6) for free. |
| Declaration extraction | Tree-sitter queries (.scm files) per language, embedded at compile time | Queries are the maintained, declarative way to extract defs/refs; Aider's query files are a reference point. |
| CLI | clap (derive API) | Subcommands: map (default), serve, diff. |
| Graph + ranking | In-house adjacency list + power-iteration PageRank (~100 lines) | No dependency needed; damping 0.85, 20 iterations, convergence check. |
| Tokenizer | tiktoken-rs (exact BPE), pluggable trait for other tokenizers | Budget enforcement uses exact counts. |
| Cache | Content-hash keyed parse results in .repomap/cache, serialized with bincode | Invalidated by grammar crate version bump; safe to delete anytime. |
| Concurrency | rayon parallel iterator over files | Parsing is embarrassingly parallel; linking/ranking is the serial tail. |
| Watch mode | notify crate (cross-platform FS events) | Feeds the same in-memory index the MCP server reads. |
| Git integration | git2 crate (libgit2 bindings) | Powers repomap diff <rev> without shelling out. |
| MCP server | stdio JSON-RPC per MCP spec (serde_json) | Reuses the watch daemon's in-memory index. |
| Distribution | cargo-dist: macOS arm64/x64, Linux x64/arm64 (musl static) | Generates Homebrew formula and curl-pipe installer; also published to crates.io. |

### 7.3 JSON schema (sketch)

```json
{ "version": 1, "repo": {"root": "...", "loc": 50312, "files": 214},
  "budget": {"target": 2048, "rendered": 1991},
  "files": [{
    "path": "src/auth/service.py", "lang": "python", "rank": 0.041,
    "symbols": [{"kind": "method", "name": "AuthService.refresh_token",
                 "sig": "(token: str) -> Token", "line": 88,
                 "visibility": "public", "refs_in": 31}],
    "imports": ["src/db/models.py", "src/config.py"] }] }
```

## 8. Success Metrics

| Metric | Definition | v0.1 target |
|---|---|---|
| Compression ratio | Repo source tokens ÷ map tokens at default budget | ≥ 100:1 on 50k-LOC repos |
| Signature accuracy | Sampled signatures matching ground truth (manual audit, 200 samples/lang) | ≥ 99% |
| Cold latency | Wall time, 50k-LOC repo, M-series laptop | ≤ 2 s |
| Warm latency | Wall time with hot cache | ≤ 200 ms |
| Agent task benchmark | Claude Code task suite (10 tasks on a 50k-LOC repo) with map in context vs. without: turns-to-completion and tokens consumed | ≥ 25% reduction in exploration tokens |
| Adoption (90 days post-launch) | GitHub stars / Homebrew installs | 500 stars (signal, not goal) |

The agent task benchmark is the metric that matters: it directly tests the product thesis. It should be built in week 1 against a baseline (no map) so every subsequent ranking/budgeting change is measurable.

## 9. Milestones

| Phase | Scope | Duration | Exit criteria |
|---|---|---|---|
| M0 — Foundation | Cargo workspace; tree-sitter + Python grammar wired; naive full map end-to-end; agent benchmark harness built and baseline (no-map) numbers recorded | 1 wk | Pipeline runs on one real repo; baseline benchmark recorded. (The OCaml plan's binding-risk spike is unnecessary in Rust — this week goes to the benchmark instead.) |
| M1 — Core (v0.1 alpha) | TS/JS + Rust grammars; import linking; PageRank; tiktoken budgeting; md + json renderers; cache; gitignore | 2 wks | FR-1, 3–7, 11–12 done; NFR-1 met; benchmark shows measurable win |
| M2 — Integration (v0.2) | MCP server; --watch; --focus personalization; cargo-dist packaging (Homebrew, crates.io, installer); docs + README with benchmark results | 2 wks | Installable by a stranger in <2 min; MCP works in Claude Code |
| M3 — Breadth (v0.3) | Go, Java, C/C++, OCaml grammars; XML renderer; repomap diff | 3 wks | Tier 2 languages pass the signature-accuracy audit |

## 10. Risks and Mitigations

| Risk | Likelihood / Impact | Mitigation |
|---|---|---|
| Tree-sitter query quality varies by grammar (extraction misses edge-case declaration forms) | Medium / Medium | Signature-accuracy audit (200 samples/lang) is a release gate; Aider's published query files are a reference baseline; queries are data, so fixes don't require code changes. |
| Ranking quality is worse than Aider's tuned heuristics | Medium / Medium | Benchmark harness from week 1; Aider's ranking approach is documented and reproducible; --focus gives users an override. |
| Aider (or Anthropic) ships a standalone repo-map first | Low–Medium / Medium | Speed is the moat for a 5-week v0.1. Even if it happens, a fast Rust binary with MCP + diff is differentiated. Worst case: excellent portfolio piece either way. |
| Rust learning curve (borrow checker friction on the graph data structures) | Medium / Low | Graph uses index-based adjacency (Vec + usize handles), the standard Rust pattern that sidesteps lifetime issues entirely; agent-assisted development is strongest in Rust. |
| Scope creep into code intelligence / search | High / Medium | Non-goals section is the contract. Anything requiring type checking or semantics is out for v1. |
| Single-maintainer bandwidth (concurrent with CCR internship, June–Aug 2026) | High / Medium | Milestones sized to ~10 hrs/wk; M1 alone is a shippable, announceable artifact. |

## 11. Open Questions

- Should the default budget be 2,048 tokens, or adaptive (e.g., 2% of repo source tokens, capped)?
- Map placement convention: committed REPOMAP.md vs. generated-on-demand vs. injected via MCP only? (Affects docs and the CI story.)
- Are docstring first-lines worth their token cost at default budget, or opt-in via --docs?
- Name check: "repomap" collides with Aider's feature name — asset or liability for discoverability? Alternatives: ctxmap, codemap, mapgen.
- Should repomap diff gate CI (fail on breaking API change) in v1, or remain informational?

## 12. Appendix: Competitive Snapshot

Aider repo-map (embedded feature, Python), universal-ctags (symbol index, no ranking/budgeting), tree-sitter CLI (primitive), Sourcegraph/SCIP (heavy server-side code intel), various "repo to prompt" scripts on GitHub (concatenate files; no compression intelligence). None combines parser-grade extraction, graph ranking, token budgeting, and single-binary distribution. That four-way combination is the product.
