# Example map gallery

Real atlas output on small, self-contained repos — so you can see what the tool
produces before installing it. Each folder holds a tiny source tree and the map
atlas generates from it, with the exact command used.

All maps here are generated with a small `--budget 400` so the committed output
stays tiny and diffs stay readable. Your own repos will use the default 2,048.

| Shape | Source | Map | Command |
| ----- | ------ | --- | ------- |
| Python service (layered: `app` → `models` → `db`) | [`python-service/`](python-service/) | [`atlas-map.md`](python-service/atlas-map.md) | `atlas python-service --budget 400` |
| TypeScript app (`api` → `user` → `types`) | [`typescript-app/`](typescript-app/) | [`atlas-map.md`](typescript-app/atlas-map.md) | `atlas typescript-app --budget 400` |
| Mixed Go + Rust + Python | [`mixed-repo/`](mixed-repo/) | [`atlas-map.md`](mixed-repo/atlas-map.md) | `atlas mixed-repo --budget 400` |

## What to notice

- **Files are ranked.** `#1` is the most central file by PageRank over the import
  graph — in `python-service`, `db.py` ranks first because both other files
  depend on it (directly or transitively).
- **Edges are shown both ways.** Each file lists what it `imports` and what it's
  `used by`, so an agent sees reverse dependencies without re-reading the tree.
- **Signatures only, no bodies.** Every function/type signature is present;
  implementation is not. That's the whole point — structure at a fraction of the
  tokens.
- **The header is the budget receipt.** `budget 400 | rendered NNN tok` shows the
  target and the actual token count.

## Refreshing the gallery

When the output format changes, regenerate every map with the pinned commands:

```sh
cargo build --release
ATLAS="$(pwd)/target/release/atlas" ./examples/gallery/refresh.sh
git diff examples/gallery   # review, then commit
```

[`refresh.sh`](refresh.sh) is the single source of truth for the commands; keep
the table above in sync with it. atlas writes a `.atlas/` cache into each folder
on run — it self-ignores (`.atlas/.gitignore`), so it never shows up in
`git status`.
