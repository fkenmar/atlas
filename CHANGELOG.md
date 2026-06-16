# Changelog

All notable user-facing changes to repomap, grouped Added/Changed/Fixed per
release ([Keep a Changelog](https://keepachangelog.com/) style). Pre-1.0
semver policy: JSON schema changes bump the minor version — see the
release-process skill. Ranking/budgeting entries must include their
benchmark delta.

## [Unreleased]

### Added
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
