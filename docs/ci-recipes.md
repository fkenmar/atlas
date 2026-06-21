# CI recipes: keeping an atlas map fresh

Two ways to wire atlas into GitHub Actions so every contributor's agent starts
with current context. Both use the standalone CLI (no MCP). For gating a PR on
**breaking public-API changes**, see [`ci-diff-gate.md`](ci-diff-gate.md) — a
different job with a different purpose.

Install atlas in CI with the pip wheel (fast, no Rust):

```yaml
- name: Install atlas
  run: pipx install --pre atlas-map
```

## Recipe 1 — informational artifact (no commit)

Generate the map on every push and upload it as a build artifact. Nothing is
committed; reviewers and agents download the map when they want it. Lowest
friction, never fails the build.

```yaml
name: atlas map
on: [push, pull_request]

jobs:
  map:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install atlas
        run: pipx install --pre atlas-map
      - name: Generate map
        run: atlas . --budget 2048 -o atlas-map.md
      - uses: actions/upload-artifact@v4
        with:
          name: atlas-map
          path: atlas-map.md
```

**Use when** you want the map available but don't want it in version control.

## Recipe 2 — committed-map freshness check

If you commit `atlas-map.md` (so it's always in the repo and in agent context),
this job fails the build when someone changes structure without regenerating the
map — keeping the committed copy honest.

```yaml
name: atlas map freshness
on: pull_request

jobs:
  freshness:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install atlas
        run: pipx install --pre atlas-map
      - name: Check the committed map is current
        run: atlas . --budget 2048 --check atlas-map.md
```

`--check` regenerates the map and compares it byte-for-byte against the committed
`atlas-map.md` instead of writing — like `rustfmt --check`. It exits `0` when the
map is current, `1` when it's stale (failing the build with a regenerate hint),
and `2` on a usage error such as a missing target. It's mutually exclusive with
`-o`/`--output`, so the job neither writes nor dirties the working tree.

**Use when** the map is committed and you want it to stay in sync with the code.

### Fixing a failed freshness check

The job fails because the committed map no longer matches the code. Regenerate it
locally with the **same command** the workflow uses and commit the result:

```sh
atlas . --budget 2048 -o atlas-map.md
git add atlas-map.md && git commit -m "Refresh atlas map"
```

Keep the budget/flags identical in the workflow, the local command, and any
[pre-commit hook](pre-commit.md), or the check will flap.

## Choosing budget / focus / lang in CI

- **`--budget`** — match what your agents actually load. 2,048 is the default;
  raise it if your team pastes maps into a large-context model, lower it to keep a
  committed map small.
- **`--lang`** — in a polyglot monorepo, generate one map per stack
  (`atlas . --lang py -o atlas-map.py.md`) so each stays focused. See
  [`monorepos.md`](monorepos.md).
- **`--focus`** — usually skip it in CI: a committed map should reflect the whole
  repo, not one contributor's working area. Use `--focus` interactively instead.
- Determinism: atlas output is byte-stable for the same input and flags, so the
  freshness check only trips on real structural change.
