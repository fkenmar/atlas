# `.atlasignore` & monorepo tuning

A good map is mostly about *what you leave out*. Vendored code, generated
clients, and giant test fixtures dilute the ranking and eat your token budget.
This guide covers the `.atlasignore` file and the flags that keep large or
multi-package repos useful.

## What atlas already skips

Before you write a single rule, atlas honors:

- **`.gitignore`** — your existing ignores apply to the map too.
- **Built-in vendored/build dirs** — `node_modules`, `target`, `dist`, `build`,
  `out`, `venv`, `__pycache__`, `vendor`, `third_party`, `site-packages`, and
  friends are always skipped, even if they aren't gitignored.
- **Hidden directories** (anything starting with `.`).

So you only need `.atlasignore` for project-specific noise that isn't already
gitignored — usually generated code you *do* commit.

## `.atlasignore` syntax

Put a `.atlasignore` file at the **repo root**. It uses a `.gitignore`-style
syntax, deliberately kept to the common, predictable subset:

| Form              | Matches                                                        | Example                  |
| ----------------- | ------------------------------------------------------------- | ------------------------ |
| `name`            | any path segment named `name` (file or dir, at any depth)     | `generated`              |
| `name/`           | directories only                                              | `__generated__/`         |
| `*.ext`           | basename glob — `*` matches within one segment                | `*.pb.go`                |
| `dir/sub`         | root-relative (anchored) path — matches `dir/sub` and below   | `packages/legacy`        |
| `dir/*.ext`       | anchored glob, one segment of `*`                             | `apps/*/dist`            |
| `# comment`       | ignored                                                       | `# build artifacts`      |

**Rules of thumb:**

- A pattern with **no `/`** matches a *basename* anywhere in the tree
  (`fixtures` ignores every `fixtures` directory).
- A pattern **containing `/`** is **anchored to the root** and matches a leading
  run of path segments (`src/gen` ignores `src/gen/**`, but not `lib/src/gen`).
- A trailing `/` restricts a rule to **directories**.

### Not supported (v1)

The matcher intentionally leaves out some `.gitignore` features. If you rely on
these, they will **not** work yet:

- **Negation** (`!keep-this`) — lines starting with `!` are skipped.
- **`**` recursive globs** — use a bare basename (matches at any depth) instead.
- **`?` and `[...]` character classes** — only `*` is a wildcard.
- **Nested ignore files** — only the **root** `.atlasignore` (and root
  `.gitignore`) is read; ignore files in subdirectories are not.

For a deep `dir/**/thing` need, prefer the basename form (`thing`) which already
matches at any depth.

## Monorepo strategies

In a monorepo, the win is usually *narrowing*, not ignoring. Three levers:

### 1. Map a subdirectory instead of the whole tree

The fastest way to a sharp map is to point atlas at the package you care about —
ranking is then computed *within* that subtree:

```sh
atlas packages/api --budget 4096
atlas apps/web
```

This is almost always better than mapping the root with a huge budget: a
focused root means PageRank centrality reflects *that* package's structure.

### 2. `--focus` to bias without excluding

When cross-package edges matter but you're working in one area, keep the whole
repo in scope and boost the paths you're touching. `--focus` seeds the ranking
so those files (and what they connect to) rise to the top of the budget:

```sh
atlas . --focus packages/api/src --focus packages/shared
```

Repeat the flag, or pass a comma-separated list. Focused paths are protected
first when the budget degrades.

### 3. `--lang` to drop whole language ecosystems

A repo with a Python backend and a TS frontend can be mapped one stack at a
time:

```sh
atlas . --lang py        # backend only
atlas . --lang ts,tsx    # frontend only
```

## Common layouts

- **`packages/*` / `apps/*` (workspaces).** Map each package directly
  (`atlas packages/<name>`), or map the root and `--focus` the active package.
  Add `dist`, `build`, and `*.d.ts` rollups to `.atlasignore` if you commit
  them.
- **Generated clients / protobufs / GraphQL.** These are committed but noisy.
  Ignore by basename or extension:

  ```
  # .atlasignore
  *.pb.go
  *_pb2.py
  __generated__/
  generated/
  ```

- **Test-heavy repos.** If fixtures or snapshots dominate the map, ignore the
  fixture dirs (they're rarely what an agent needs to navigate):

  ```
  fixtures/
  __snapshots__/
  testdata/
  ```

  Keep real test *sources* if they document the public API; drop only bulk data.

## How ignoring affects ranking and budget

Ignoring isn't just cosmetic — it changes the map you get:

- **Ranking.** atlas ranks files by centrality in the import graph. A vendored
  or generated directory with many incoming edges can crowd out your real code.
  Removing it lets first-party modules rise to the top.
- **Budget.** The token budget is fixed (2,048 by default). Every ignored file
  is budget returned to the files that matter — so trimming noise often surfaces
  *more* real signatures at the same `--budget`.

If a file you *expected* is missing, it's usually an unsupported language, a
gitignored/vendored directory, or an over-broad `.atlasignore` rule — check with
`atlas explain <path>` or by temporarily narrowing the ignore file. See the
[README troubleshooting](../README.md#troubleshooting) section for the common
cases.
