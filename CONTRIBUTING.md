# Contributing to atlas

Thanks for your interest! atlas is a small, focused tool, and contributions of
all sizes are welcome — bug reports, language fixes, docs, and features alike.

## Quick start

```
git clone https://github.com/fkenmar/atlas
cd atlas
cargo build
cargo test
```

Run it against any repo to see the map:

```
cargo run -- path/to/some/repo --budget 2048
```

## Good first contributions

You don't need to understand the whole pipeline to help. The easiest entry
points, and the smallest test to run for each:

| Kind of change                          | Touch                                                | Smallest test                         |
| --------------------------------------- | ---------------------------------------------------- | ------------------------------------- |
| **Docs only** (README, `docs/`, guides) | Markdown files                                       | none — just proofread your links      |
| **Query / extraction fix**              | `queries/<lang>/tags.scm`, `tests/queries/fixtures/` | `cargo test query_`                   |
| **CLI behavior / exit codes**           | `src/cli.rs`, `tests/*_cli.rs`                       | `cargo test --test map_cli` (or the relevant `*_cli`) |

Browse the
[`good first issue`](https://github.com/fkenmar/atlas/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
label for curated starter tickets — mostly docs, query fixtures, and small CLI
tests. The [language support matrix](docs/languages.md) and
[exit-code contract](docs/exit-codes.md) are good orientation before a query or
CLI change.

> **Not a first issue:** anything that changes **ranking or budgeting**. Those
> are gated on the [benchmark](benchmark/README.md) (measured, not merged on
> intuition) and assume familiarity with the pipeline — see the
> [self-improvement loop](docs/SELF_IMPROVEMENT.md). Pick one up once you've
> landed a smaller change.

## Before you open a PR

Every change must pass the same gate CI runs — all green:

```
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

A few conventions worth knowing (the full set is in [CLAUDE.md](CLAUDE.md)):

- **No `.unwrap()` / `.expect()`** outside tests; use `anyhow` in the binary and
  `thiserror` for library error types.
- **Deterministic output** — iterate sorted collections; never rely on `HashMap`
  order. The same input must always produce byte-identical output.
- **Unparseable files are skipped and counted, never a panic.**
- **New dependencies need a maintainer's OK first** — please ask in the issue
  before adding one, and justify it in the PR description (see
  [Dependencies](#dependencies)).
- **Ranking or budgeting changes** are measured against the benchmark
  ([benchmark/README.md](benchmark/README.md)), not merged on intuition — note the
  delta in your PR.

## Adding a language

atlas extracts symbols with tree-sitter queries. To add or fix one:

1. Edit `queries/<lang>/tags.scm` (the capture contract — which `@definition.*`
   tags map to which `SymbolKind` — is documented in
   `.claude/skills/tree-sitter-queries/SKILL.md`).
2. Add a fixture under `tests/queries/fixtures/` and a `query_*` snapshot test.
3. `cargo test query_` — green before you push.

**Don't guess AST node names.** Tree-sitter node names (`function_definition`,
`type_spec`, …) vary by grammar; a guessed name silently matches nothing. Print
the real tree for your fixture and copy names from it:

```
tree-sitter parse tests/queries/fixtures/<lang>.<ext>
```

(install once with `cargo install tree-sitter-cli`). The grammar's own
`node-types.json` and its upstream `queries/tags.scm` are the other reliable
sources. The [language support matrix](docs/languages.md) lists what each
existing language already extracts.

## Dependencies

atlas ships as a single fast binary, so every dependency is weighed against the
cost it adds. **Ask before adding one** — open or comment on an issue first, and
when you do, justify it in the PR against this bar:

- **Runtime impact** — does it add startup cost or slow the hot path? atlas
  targets ≤2 s cold / ≤200 ms warm on 50k LOC (NFR-1).
- **Binary size** — a heavy transitive tree bloats the prebuilt binaries users
  download.
- **Supply-chain risk** — more crates (and their transitive deps) is more to
  trust and audit. Prefer well-maintained, widely-used crates; avoid anything
  unmaintained.
- **Maintenance burden** — major-version churn, MSRV bumps, and platform quirks
  all land on us.
- **Alternatives** — can the std library, an existing dependency, or ~30 lines
  of code do it? Often yes.

Some categories get extra scrutiny:

- **Parser grammars** (`tree-sitter-*`) — adding one is how new languages land,
  but pin the version and add a fixture + snapshot test in the same PR (see
  [Adding a language](#adding-a-language)). Grammar version bumps can shift
  extraction, so re-snapshot deliberately.
- **Release / build tooling** (cargo-dist, CI actions) — dev-time only, but still
  affects what users download; flag size or provenance changes.
- **MCP / server dependencies** — the `serve --mcp` path must not pull weight
  into the core map pipeline; keep server-only deps out of the default build
  where feasible.
- **Dev-only tools** (`[dev-dependencies]`) — a lower bar since they don't ship
  in the binary, but they still cost CI time and trust.

When in doubt, propose it in the issue before writing the code.

## Scope

atlas is purely a **structural** map: signatures, types, and import edges, ranked
and budgeted. It is intentionally *not* a semantic search engine, an LSP, an
editor, or a code generator. Out-of-scope ideas are parked in
[ideas.md](ideas.md) rather than dropped — feel free to add yours there.

## Reporting bugs

Open an issue with the command you ran, the repo (or a minimal snippet) it ran
on, and what you expected versus what you got. A wrong or missing symbol in the
map is a great, actionable bug report.

MIT © Kenmar
