# CI gate for public API changes (`atlas diff --exit-code`)

`atlas diff` is informational by default (always exits 0). Pass `--exit-code` to
make it a **CI gate**: it exits non-zero (1) when the diff contains a *breaking*
change — a removed or signature-changed **public** symbol or file (the
`breaking` severity from the diff's severity summary). Additions, kind-changes,
moves, and private-only changes do **not** fail the gate.

```sh
# Fail the build if the public API surface has a breaking change:
atlas diff "$BASE_DIR" "$HEAD_DIR" --no-private --exit-code
```

Pair it with `--no-private` to gate on the public surface only (private churn
won't trip it). `--format json` works too if a later step parses the per-change
`severity` field.

## Sample GitHub Actions workflow

This checks out the PR base and head into two worktrees and gates on breaking
public-API changes — unless the PR carries an `api-break-ok` label (the override
escape hatch).

```yaml
name: API surface check
on: pull_request

jobs:
  atlas-diff:
    runs-on: ubuntu-latest
    # Skip the gate when the change is an intentional, reviewed API break.
    if: ${{ !contains(github.event.pull_request.labels.*.name, 'api-break-ok') }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install atlas
        run: pipx install --pre atlas-map
      - name: Materialize base and head
        run: |
          git worktree add ../base "${{ github.event.pull_request.base.sha }}"
          git worktree add ../head "${{ github.sha }}"
      - name: Gate on breaking public-API changes
        run: atlas diff ../base ../head --no-private --exit-code
```

## When to gate (and when it's too noisy)

**Good fits**

- Libraries / SDKs / public APIs where a removed or changed public signature
  breaks downstream consumers.
- Repos with a stable plugin or extension surface.

**Likely too noisy — keep it informational instead**

- Apps/services with no external consumers, where internal refactors freely
  change "public" symbols.
- Early-stage code with a fast-moving surface (run `atlas diff` for review
  context, but don't fail the build).

The gate is a *heuristic* (it reads syntax, not types) — use it as a prompt for a
human look, with the `api-break-ok` label as the reviewed-and-intended override.
