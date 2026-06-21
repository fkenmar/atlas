# Migrating from repomap to atlas

> **Historical note.** This project was originally named **repomap** and was
> renamed to **atlas**. If you started using the tool after the rename, you can
> ignore this page. It exists for early users who still have repomap-era names
> or files lying around. It will be retired once those names are no longer
> common.

## Name map

Everything that used to say `repomap` now says `atlas`:

| repomap-era name            | atlas equivalent          | Notes                                            |
| --------------------------- | ------------------------- | ------------------------------------------------ |
| `repomap` (binary/command)  | `atlas`                   | Same flags and pipeline; just a renamed binary.  |
| `repomap` (Cargo crate)     | `atlas`                   | `cargo install --path .` builds `atlas`.         |
| `repomap` (PyPI package)    | `atlas-map`               | `pipx install --pre atlas-map`; command is `atlas`. The PyPI name has a suffix because `atlas` was taken. |
| `.repomapignore`            | `.atlasignore`            | Same syntax (a `.gitignore`-style matcher).      |
| `.repomap/` (cache dir)     | `.atlas/`                 | Parse cache plus a self-written `.gitignore`.    |
| `.repomap/cache`            | `.atlas/cache`            | bincode parse cache, safe to delete.             |
| `REPOMAP.md` / `repomap.md` | `atlas-map.md`            | Just a conventional output filename — you pick it with `-o`. Nothing requires this exact name. |

The output format header also changed from `# repomap: …` to `# atlas: …`.

## What to do with stale files

The renamed tool does **not** read any `repomap`-era files, so leftovers are
harmless but useless. To clean up:

- **Stale `.repomap/` cache directory.** Delete it — atlas writes its own
  `.atlas/` and never looks at `.repomap/`:

  ```
  rm -rf .repomap
  ```

  (atlas already keeps its own cache out of git by writing `.atlas/.gitignore`;
  run `atlas cache info` to see the current cache path and size.)

- **`.repomapignore` rules you still want.** atlas ignores `.repomapignore`
  entirely. Rename it so atlas picks the rules up:

  ```
  git mv .repomapignore .atlasignore   # or: mv .repomapignore .atlasignore
  ```

  See [`docs/monorepos.md`](monorepos.md) for `.atlasignore` syntax and tuning.

- **A committed `REPOMAP.md`.** Regenerate it under the new name and drop the
  old one:

  ```
  atlas . -o atlas-map.md
  git rm REPOMAP.md
  ```

- **CI / scripts / pre-commit hooks** that call `repomap …`. Replace the command
  with `atlas …` — the flags are unchanged. If you install via pip, the package
  is `atlas-map` but the command on `PATH` is still `atlas`.

## Troubleshooting

- **`command not found: repomap`.** Expected — the binary is `atlas` now.
  Reinstall (see the [README](../README.md#install)) and call `atlas`.
- **My old ignore rules stopped working.** They were in `.repomapignore`, which
  atlas does not read. Rename it to `.atlasignore` (see above).
- **`git status` shows a `.repomap/` directory.** That's a leftover cache from
  the old tool; `rm -rf .repomap`. The current cache lives in `.atlas/` and is
  self-ignored.

If something still references a `repomap` name that isn't covered here, please
[open an issue](https://github.com/fkenmar/atlas/issues/new).
