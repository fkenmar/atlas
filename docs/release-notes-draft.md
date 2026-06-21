# Release-notes draft (value-first)

`CHANGELOG.md` is the complete, technical record. GitHub *release notes* are the
first thing a newcomer reads when they find the repo, so they should lead with
the value and the honest benchmark, not a commit dump. Paste this into the
release (adjust the version) and let the auto-generated commit list sit below it.

---

## atlas v0.2.1-alpha

**Give your AI coding agent a map of your repo so it stops burning tokens just
finding its way around.** atlas compiles a codebase into a compact, ranked,
token-budgeted structural map — every signature, type, and import edge, no
function bodies — that an agent reads in one shot.

### Why you might want it

In a comprehension benchmark (20 verified questions on a real repo), agents
answered with **identical accuracy — 20/20 with and without the map — using ~65%
fewer tokens** (85,670 → 29,781) and usually in **1 turn instead of 3**. You can
[reproduce it yourself](https://github.com/fkenmar/atlas/blob/main/benchmark/README.md#reproduce-the-headline-number).

*Honest caveat:* on open-ended edit tasks the token numbers are too noisy to
claim a win — turns trend down ~25%, but we don't headline what we can't measure.

### What's in it

- Ranked map (PageRank over the import graph), packed to a token budget (default
  ~2,048) with a graceful degradation ladder.
- Markdown / JSON / XML output; `--focus`, `--lang`, `--no-private`, `--budget`.
- `atlas diff` — structural delta between two trees or git revisions, with an
  optional `--exit-code` CI gate.
- `atlas serve --mcp` — read-only MCP stdio server (`get_map`, `get_symbol`).
- Languages: Python, TypeScript/JavaScript, Rust, Go, Java, C/C++.
- Local, offline, deterministic, no telemetry.

### Install

```sh
pipx install --pre atlas-map     # or: pip install --pre atlas-map
# prebuilt binaries (macOS/Linux/Windows) are attached below
```

### Status

Alpha — the core works and is benchmark-tested, but the CLI and output format may
still change; pin a version if you depend on the output. Feedback from people
using Claude Code, Cursor, Codex, or Aider is especially welcome.

---

**Reuse:** keep the benchmark wording in sync with
[`benchmark/history.md`](../benchmark/history.md) (the launch checklist requires
this), and never add an edit-task token claim without a result that clears the
variance gate in `benchmark/README.md`.
