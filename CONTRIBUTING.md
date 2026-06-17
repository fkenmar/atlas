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
  before adding one.
- **Ranking or budgeting changes** are measured against the benchmark
  ([benchmark/README.md](benchmark/README.md)), not merged on intuition — note the
  delta in your PR.

## Adding a language

atlas extracts symbols with tree-sitter queries. To add or fix one:

1. Edit `queries/<lang>/tags.scm` (the capture contract is documented in
   `.claude/skills/tree-sitter-queries/SKILL.md`).
2. Add a fixture under `tests/queries/fixtures/` and a `query_*` snapshot test.
3. `cargo test query_` — green before you push.

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
