# atlas

> Compile a codebase into a token-budgeted structural map for LLM coding agents.

**Status: alpha (M1 — Core, built).** The full pipeline runs end-to-end for Python, Rust, and TypeScript/JavaScript: `repomap <path>` emits a token-budgeted, PageRank-ranked map (pytest's 92k LOC → a 2,048-token map in 0.25 s cold). All M1 functional requirements are done — TS/JS + Rust grammars, import linking, personalized PageRank, exact-BPE budgeting with a degradation ladder, Markdown + JSON renderers, an incremental cache, and `.gitignore`/`.repomapignore` handling. See [STATUS.md](STATUS.md) for the live board, [CHANGELOG.md](CHANGELOG.md) for what's landed, and [docs/PRD.md](docs/PRD.md) for the spec.

> Being renamed **repomap → atlas** (title + repo done); the CLI binary is still `repomap` until the package rename lands.

atlas parses a repository with tree-sitter and emits a compressed structural map — every function signature, type definition, class/struct field, exported symbol, and import edge, with zero function bodies — packed into a token budget (default 2,048) by personalized PageRank over the import/reference graph. A 50k-LOC repo becomes a ~2k-token summary an agent can hold entirely in context, so it stops burning turns on exploration.

## Pipeline

```
discover → parse → link → rank → budget → render
```

One stage, one module — see [CLAUDE.md](CLAUDE.md) for the stage-to-file map.

## Usage

Working today (M1):

```
$ repomap .                          # map cwd, 2048-token budget, markdown to stdout
$ repomap . --budget 4096 --format json
$ repomap . --focus src/auth/        # boost ranking for the files you're editing
$ repomap . --lang py,rs             # restrict languages
$ repomap . --no-private             # public API surface only
```

Planned (M2+): `repomap serve --mcp` (MCP server: `get_map`, `get_symbol`) and `repomap diff HEAD~5` (API-surface diff between revisions).

Tier 1 languages: TypeScript/JavaScript, Python, Rust. Tier 2 (later): Go, Java, C/C++, OCaml.

## Benchmark

The product thesis is measured, not assumed: a 10-task Claude Code benchmark ([benchmark/README.md](benchmark/README.md)) compares agent exploration tokens and turns-to-completion with the map in context vs. without. v0.1 target: **≥25% reduction in exploration tokens**.

**No-map baseline recorded 2026-06-12** (pytest 8.2.0, claude-sonnet-4-6, 3 runs/task, medians): an agent without a map spends **902,555 tokens / 22 turns** adding a field threaded through three sites, and **369,461 tokens / 14 turns** finding an existing utility instead of reimplementing it (all runs passed; high run variance noted in STATUS.md).

**Preliminary with-map probe (same day, unofficial):** injecting even the naive unbudgeted map cut turns **41–43%** on both tasks — but raised total tokens, because the ~81k-token naive map is re-read from cache every turn (92% of the bill). Navigation value proven; converting it into token savings is what M1's ranking + budgeting stage is for.

**M1 with-map results (2026-06-16, refined metric — `exploration_tokens` = input-side tokens before the first edit, medians over passing, non-capped runs; see [benchmark/history.md](benchmark/history.md)).** The budgeted map's effect is task-type-dependent:

- **Locate-a-utility task: −78% exploration tokens** (920k vs 4.20M, same-run with-map vs without) — at the v0.1 target and within reach of an aspirational 80%. The map surfaces the existing helper so the agent skips the tree walk (47 → 15 turns).
- **Comprehension Q&A: −45% tokens at equal (6/6) accuracy** — the map never trades correctness for speed.
- **Multi-site-edit task:** the generic map can *hurt* here (the agent over-explores) — it can't replace grep for finding all edit sites, and a denser map adds per-turn cost. High variance; N≥5 confirmation and reverse-dependency ("used by") info are the open work.

Honest aggregate so far: a strong, near-target win where the map is designed to help, not yet a uniform reduction across all task types. Releases are blocked on this section being current.

## Development

```
cargo build && cargo test
```

- Conventions, workflow rules, and the scope contract: [CLAUDE.md](CLAUDE.md)
- Architecture decisions: [docs/adr/](docs/adr/)
- Development runs as a measured loop, not ad-hoc prompting: [docs/SELF_IMPROVEMENT.md](docs/SELF_IMPROVEMENT.md) — `/improve` per iteration, keep-or-revert decided by benchmark stats, every measured change appended to [benchmark/history.md](benchmark/history.md)
- Distribution (M2): single static binaries via cargo-dist — Homebrew, `cargo install repomap`, curl-pipe installer

MIT © Kenmar
