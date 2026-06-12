# repomap

Standalone Rust CLI that compiles a codebase into a token-budgeted structural map for LLM coding agents: every signature, type, export, and import edge — zero function bodies — packed into ~2k tokens by graph ranking. Full spec: docs/PRD.md. Development is benchmark-driven and milestone-gated; the agent-task benchmark (benchmark/README.md) is the arbiter of all ranking/budgeting changes.

## Pipeline (one stage = one module)

| Stage | What it does | Owner |
|---|---|---|
| discover | walk tree; .gitignore, .repomapignore, vendored defaults | src/discover.rs |
| parse | tree-sitter per file; extract declarations via queries/<lang>/tags.scm | src/parse.rs + src/lang/ |
| link | imports + defs↔refs into a directed file/symbol graph | src/link.rs |
| rank | personalized PageRank; --focus seeds the personalization vector | src/rank.rs |
| budget | greedy pack to N tokens; degradation ladder; exact BPE counts | src/budget.rs |
| render | markdown / json / xml; deterministic output | src/render/ |

src/cache.rs sits under parse: content-hash keyed, bincode, `.repomap/cache`. src/cli.rs drives the stages.

## Conventions

- Graphs are index-based: `Vec<Node>` + `usize` handles. No references, lifetimes, `Rc`, or `RefCell` in graph structures (ADR 0002).
- Errors: `anyhow` in binary code, `thiserror` for library error types.
- No `.unwrap()` / `.expect()` outside tests.
- Unparseable files are skipped and counted, never a panic (FR-12).
- Output is deterministic (NFR-4): iterate sorted collections — BTreeMap, or sort before render. Never rely on HashMap iteration order.
- All new dependencies require asking the maintainer first — even ones already named in the PRD.

## Workflow rules

- Before declaring any task done: `cargo fmt && cargo clippy -- -D warnings && cargo test` — all green.
- Ranking/budgeting changes are not done until `/bench` has run and the delta vs. benchmark/baseline.json is reported.
- Out-of-scope ideas go to ideas.md, never into code.
- When a NOW task completes, update STATUS.md.
- Significant architecture decisions get an ADR via `/adr`. ADRs are append-only; never edit an existing one.

## Scope contract — decline these

Hard non-goals (PRD §3.2). If asked for one, decline plainly, park it in ideas.md, and suggest the in-scope alternative if one exists (`/scope-check` does this):

- Semantic search or embeddings — repomap is purely structural.
- Full code intelligence (go-to-definition, rename) — that's an LSP's job.
- IDE plugins or GUI — CLI and MCP only.
- Editing or generating code — read-only tool.
- Languages beyond the Tier 1/2 set (Section 6) — breadth comes after the ranking and budgeting core is right.

## Where things are

- STATUS.md — current milestone, exit criteria, NOW/NEXT/NOT-YET board. Surfaced by the SessionStart hook.
- benchmark/README.md — the benchmark protocol; benchmark/history.md — append-only stats ledger of every measured change.
- docs/SELF_IMPROVEMENT.md — the autonomous improvement loop: `/improve` for one measured iteration, `/loop /improve` for continuous; keep-or-revert is decided by stats, never vibes.
- docs/adr/ — past decisions; read before redesigning anything.
- docs/PRD.md — requirements, milestones, risks, open questions.
