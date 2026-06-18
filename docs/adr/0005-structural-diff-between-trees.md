# ADR 0005 — `atlas diff` compares two trees via a git-style router, reusing parse (not budget)

## Context

Issue #12 (M3) asks for a structural diff: "what changed at the signature/type/edge
level between two revisions" so an agent sees the delta without re-reading the tree.
Three decisions had to be made, none obvious:

1. **Input model.** The PRD frames it as "between revisions," which suggests reading
   git history. Doing that in-process needs `git2` (a new crate, gated by the
   ask-first dependency rule and not yet approved), or shelling out to the `git`
   binary (requires a repo, harder to test deterministically).
2. **CLI shape.** The existing `Cli` is a flat clap-derive struct with a default
   positional `path` — `atlas .` is the implicit `map` command, and a large test
   suite asserts against its fields. Introducing a real clap `#[command(subcommand)]`
   alongside an optional top-level positional reintroduces clap's
   "is `atlas src` the dir `src` or an unknown subcommand?" ambiguity, risking the
   *primary* use case, and rewrites every `cli.format`-style test to `cli.map.format`.
3. **What to diff.** The budgeted map (rank + token budget) only contains the
   surviving subset of symbols; a diff that silently dropped low-rank changes would
   be wrong.

## Decision

**`atlas diff <old> <new>` compares two directory trees** (paths), not git revisions.
A user diffs revisions by materializing them first (e.g. `git worktree add`); the
path-vs-path engine is the general primitive and "or equivalent" per the issue's
done-criteria. Git-revision convenience is a deferred follow-up, layered on once
`git2` is approved — it does not change the engine.

**Routing is a git-style manual dispatch, not a clap subcommand.** `run()` checks
`argv[1] == "diff"` *before* `Cli::parse()` and hands the remaining args to a
dedicated `DiffArgs` clap parser. The map `Cli` and all its tests are untouched.
The pre-existing "planned command" stub drops `diff`, keeping only `serve`. Trade-off:
`atlas diff` is not listed by `atlas --help`'s arg summary, so it is advertised in the
`EXAMPLES`/after-help block and README instead.

**The diff runs on raw parse output** (`discover` → `parse_all`), skipping `rank` and
`budget`, so every changed symbol is reported. A new `src/diff.rs` builds a
`StructuralDiff` of: added/removed files; and per common file, added/removed symbols,
**changed signatures** (matched by `(kind, name)`, signature differs), and
added/removed import edges. Output is a deterministic Markdown delta (sorted
collections, NFR-4), rendered by `render/diff.rs`. JSON/XML diff output is a deferred
follow-up.

## Consequences

- **No new dependency**, no benchmark impact (diff is a separate path that never
  touches ranking/budgeting), and zero risk to the map command's parsing — the
  surgical router keeps the existing 79-test gate intact.
- **Symbol identity is `(kind, name)`.** Overloads/duplicate names within one file
  fall back to set-based add/remove of signatures rather than a clean "changed" pair;
  acceptable for v1, revisit if it proves noisy.
- **`atlas diff` is less discoverable** than a first-class subcommand (absent from the
  top-level help summary); mitigated by the examples block and README, and reversible
  later by promoting it to a clap subcommand if the map `Cli` is restructured.
- **Diffing two large trees parses both fully** (no cache sharing across the two
  roots). Fine at current scale (NFR-1 cold parse is well under budget); revisit if
  diff latency becomes a concern.
