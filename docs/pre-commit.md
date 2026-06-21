# Pre-commit hook: keep `atlas-map.md` current

If you commit a map so every contributor and agent starts oriented, a pre-commit
hook keeps it fresh with zero ceremony. Warm re-runs are cache-backed (~80 ms on
a 50k-LOC repo), so the hook is cheap enough to run on every commit.

Pick **one** of the two approaches below.

## Option A — plain Git hook

Create `.git/hooks/pre-commit` (and `chmod +x` it):

```sh
#!/usr/bin/env sh
# Regenerate the committed atlas map and stage it if it changed.
set -e

atlas . --budget 2048 -o atlas-map.md
git add atlas-map.md
```

This regenerates and stages the map on every commit. `.git/hooks/` isn't shared,
so document it in your README or ship a setup script
(`cp scripts/pre-commit .git/hooks/ && chmod +x .git/hooks/pre-commit`).

Prefer a **check** over auto-staging? Use `--check` to fail the commit when the
map is stale, without writing anything — like `rustfmt --check`, it regenerates
the map and compares it byte-for-byte against the committed file, exiting `1` if
they differ (and `2` on a usage error such as a missing target):

```sh
#!/usr/bin/env sh
set -e
# Exit 1 (commit blocked) if atlas-map.md no longer matches the code.
atlas . --budget 2048 --check atlas-map.md
```

`--check` is mutually exclusive with `-o`/`--output` (one verifies, the other
writes). Keep the `--budget`/flags identical to the command that *generates* the
map, or the check will flap.

## Option B — pre-commit framework

If you already use [pre-commit](https://pre-commit.com), add a local hook to
`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: atlas-map
        name: Refresh atlas map
        entry: atlas . --budget 2048 -o atlas-map.md
        language: system
        pass_filenames: false
        always_run: true
        stages: [pre-commit]
```

`pass_filenames: false` and `always_run: true` make it regenerate the whole-repo
map regardless of which files changed. After the hook writes `atlas-map.md`,
`git add` it (pre-commit reports modified files; stage and re-commit).

## Don't commit the `.atlas/` cache

Running atlas writes a parse cache to `.atlas/` at the repo root. atlas
**self-ignores** it — it writes `.atlas/.gitignore` containing `*` — so it won't
show in `git status` and the hook won't stage it. If you want a belt-and-braces
entry, add to your `.gitignore`:

```
.atlas/
```

## Performance expectations

- **Cold** (first run, or after `cargo`/dependency churn): up to ~2 s on 50k LOC.
- **Warm** (typical commit, cache hit): ~80–200 ms.

Only changed files are re-parsed, so a one-file commit is effectively instant.

## When *not* to use a committed map

A committed map is great for shared agent context, but it isn't always the right
tool:

- **Solo / on-demand work** — just run `atlas .` when you need a map; skip the
  commit churn.
- **Very fast-moving structure** — frequent renames make the map diff noisy in
  every PR. Prefer the [CI artifact recipe](ci-recipes.md) or generate on demand.
- **You want a live map in-agent** — use the MCP server's `get_map` tool instead
  ([`CLAUDE_CODE_MCP.md`](CLAUDE_CODE_MCP.md)), which never commits anything.
