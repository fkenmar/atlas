# ADR 0007 — `atlas diff` resolves git revisions via the git CLI, not git2

## Context

ADR 0005 shipped `atlas diff <old> <new>` over two directory paths and deferred
the headline `atlas diff HEAD~1 HEAD` revision UX, noting it needs either the
`git2` crate (gated by the ask-first dependency rule, unapproved) or shelling out
to the `git` binary. Issue #18 asks for the revision form. The path-vs-path
engine is unchanged; the only new work is turning a revision into a tree on disk.

## Decision

Each `atlas diff` positional is **auto-detected**: if it names an existing
directory it is used as a path (unchanged behavior); otherwise it is treated as a
git revision and **materialized via the `git` CLI** — no new crate. So
`atlas diff old/ new/`, `atlas diff HEAD~1 HEAD`, and the mix
`atlas diff HEAD~1 ./new` all work. Both positionals stay required (the
one-arg "diff against the working tree" form is a later follow-up), so the
existing CLI surface and its tests are unchanged.

Materialization uses `git worktree add --detach <tmpdir> <rev>` into a unique
temp directory, which is parsed like any path and then removed with
`git worktree remove --force`. Chosen over `git archive | tar` (no `tar`
dependency, no manual stdout piping) and over `git2` (no new crate). A revision
that doesn't resolve, or running outside a git repo, is a clean exit-2 error.
Temp worktrees are cleaned up after the diff (best-effort on the error path;
`git worktree prune` reclaims any leak).

## Consequences

- The revision form needs `git` on `PATH` and a repository in the current
  directory; the path-vs-path form keeps working with neither. The error message
  says so when resolution fails.
- A transient worktree is added to the repo's worktree list during the run and
  removed after — a side effect path-vs-path never had, but bounded and
  self-cleaning. `git archive` would avoid it entirely; revisit if the worktree
  churn proves noticeable on large repos.
- No new dependency; no benchmark impact (diff never touches rank/budget).
- Determinism holds: a revision materializes to a fixed tree, so the diff is
  reproducible.
- Symlink/`.gitignore` semantics follow whatever `git worktree` checks out
  (tracked files at that commit) — which is exactly the structural state wanted.
