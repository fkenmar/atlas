# Exit codes & error taxonomy

atlas uses a small, stable set of process exit codes so scripts and CI can react
to *what* happened without scraping stderr. There are exactly three:

| Code | Meaning            | When                                                                 |
| ---- | ------------------ | -------------------------------------------------------------------- |
| `0`  | Success            | The command produced its output normally.                            |
| `1`  | Operational error  | atlas started fine but could not produce a result, or hit an I/O / internal failure. |
| `2`  | Usage error        | The invocation or environment is wrong; atlas did no work.           |

Diagnostics always go to **stderr**; the map / diff / JSON output goes to
**stdout**. So `atlas . > map.md` leaves `map.md` empty on a non-zero exit, and
you can read the reason from stderr.

## 0 — success

The command completed. This includes maps that skipped files:

- **Parse warnings never fail the run.** Unparseable or unsupported files are
  skipped and *counted*, not fatal (FR-12). A map that skipped 3 files still
  exits `0`; the skip count is reported in `--timings` and the map header.
- **`atlas diff` exits `0` even when things changed.** The diff is informational
  by default. To make a *breaking* structural change fail (for CI), add
  `--exit-code` — that turns a breaking change into exit `1` (see below).

## 1 — operational error

atlas was invoked correctly but couldn't deliver a result:

- **No supported source files** under the given path (`atlas .`, `atlas explain`).
  The message lists the file extensions atlas actually saw, to flag a wrong root
  or an unsupported language.
- **`--lang` matched none** of the discovered files.
- **Tokenizer failed to initialize** (budget stage).
- **Writing the `--output` file failed** (permissions, full disk, …). The
  destination is left untouched — atlas writes through a temp file and renames.
- **`atlas serve --mcp` server failed** at runtime.
- **`atlas explain <path>`** where `<path>` exists under the root but isn't a
  mapped source file.
- **`atlas diff --exit-code`** found a **breaking change** — a removed or
  signature-changed public symbol or file. Pair with `--no-private` to gate on
  the public surface only. (Without `--exit-code`, this is still exit `0`.)

## 2 — usage error

The command line or environment is wrong, so atlas did nothing:

- **Bad path** — not found, permission denied, or not a directory (atlas maps a
  repo root, not a single file).
- **`--budget 0`** (the budget must be at least 1 token).
- **`--for-agent` with a non-Markdown format** (`--format json` / `xml`).
- **Unknown `--lang` value** (the message lists supported extensions).
- **`atlas diff <rev>`** where the revision can't be resolved, or `git` isn't
  available to check it out.
- **`atlas explain <path>`** where `<path>` is outside the repo root.
- **`atlas cache clean` without `--force`** (a safety stop — it prints what it
  *would* remove).
- **`atlas serve` without `--mcp`** (no other server mode exists yet).
- **Argument-parsing errors** — unknown flag, missing value, invalid enum. These
  come from the CLI parser, which also exits `2`.

## Using exit codes in CI

- **Fail a job when a repo maps to nothing.** A plain `atlas .` returns `1` if no
  supported files are found, so a bare invocation already gates "did we map
  anything?":

  ```sh
  atlas . -o atlas-map.md || exit 1   # 1 = nothing mapped, 2 = bad invocation
  ```

- **Treat parse warnings as non-fatal.** They are, by design — atlas exits `0`
  with skipped files counted. If you want to *surface* them, run with
  `--timings` and inspect stderr; the exit code stays `0`.

- **Gate a PR on breaking API changes.** Use the structural diff with
  `--exit-code` (and usually `--no-private`) so the job fails only on a removed
  or changed public symbol:

  ```sh
  atlas diff origin/main . --no-private --exit-code
  ```

  See [`docs/ci-diff-gate.md`](ci-diff-gate.md) for a full workflow.

- **Distinguish "broken setup" from "empty result."** `2` means *you* (or the
  config) called atlas wrong — a bad flag or path; treat it as a hard failure to
  fix the pipeline. `1` means atlas ran but had nothing to produce — often a
  legitimate signal about the repo, not the command.
