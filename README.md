<div align="center">

<img src="assets/atlas-logo.svg" width="120" alt="atlas logo">

# atlas

**Turn any codebase into a compact map your AI coding agent can read in one shot — so it stops burning tokens exploring.**

<p>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-4F46E5?style=flat-square" alt="License: MIT"></a>
  <a href="https://github.com/fkenmar/atlas/releases"><img src="https://img.shields.io/badge/release-v0.2.0--alpha-4F46E5?style=flat-square" alt="Release"></a>
  <a href="https://github.com/fkenmar/atlas/actions/workflows/ci.yml"><img src="https://github.com/fkenmar/atlas/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/built%20with-Rust-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Built with Rust"></a>
  <img src="https://img.shields.io/badge/languages-Py%20%C2%B7%20TS%20%C2%B7%20Rust-F59E0B?style=flat-square" alt="Languages: Py · TS · Rust">
</p>

<sub>
  <a href="#install"><b>Install</b></a> ·
  <a href="#use"><b>Use</b></a> ·
  <a href="#use-it-with-your-ai-agent"><b>Use it with your agent</b></a> ·
  <a href="#why-it-works"><b>Why it works</b></a> ·
  <a href="#troubleshooting"><b>Troubleshooting</b></a>
</sub>

</div>

---

When an AI coding agent works in your repo, it spends most of its effort just figuring out where things are: opening file after file to learn the layout. **atlas** does that once and hands the agent a single ~2,000-token map — every function signature, type, and import, ranked by importance, with no function bodies. The agent gets its bearings immediately and gets to work.

In our benchmark, agents given an atlas map answered structural questions about a codebase using **~65% fewer tokens at identical accuracy** (20/20 correct *with* and *without* the map) — typically resolving in a **single turn instead of three**. On open-ended edit tasks the map cuts turns too, though token savings there vary by task.

## What it looks like

Run `atlas src --budget 600` on this repo and it prints:

```
# atlas: src (3740 LOC, 16 files) | budget 600 | rendered 585 tok

## cache.rs (#1 — imported by 1 file(s))
pub struct Cache
    pub fn open(&Path) -> Cache
    pub fn get(&mut self, &str, u64) -> Option<ParsedFile>
    pub fn save(self)
pub fn content_hash(&str) -> u64
imports: parse.rs
used by: parse.rs
```

Files are ordered by importance (a PageRank over the import graph), `#1` being the most central. Each file shows its public symbols, what it imports, and what depends on it — everything an agent needs to navigate, nothing it doesn't. The header reports the budget and the actual `rendered` token count.

**What it maps:** Python (`.py`, `.pyi`), TypeScript/JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, …), and Rust (`.rs`). It honors your `.gitignore` and an optional `.atlasignore`, and always skips hidden directories and common vendored/build folders (`node_modules`, `target`, `dist`, `build`, `venv`, `__pycache__`, `vendor`, …). If a file you expected isn't there, it's almost always an unsupported language or a skipped directory.

---

## Install

**pip / pipx** — for the Python crowd, no Rust required:

```
pipx install --pre atlas-map     # or: pip install --pre atlas-map
```

atlas is a Rust binary, not a Python package — the wheel just drops the native `atlas` command onto your PATH (the same way `ruff` and `uv` ship). The PyPI distribution is named `atlas-map` because `atlas` was taken; the command you run is still `atlas`. While atlas is in alpha the `--pre` flag is required.

**Prebuilt binary** — no Rust required (macOS & Linux):

```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fkenmar/atlas/releases/download/v0.2.0-alpha/atlas-installer.sh | sh
```

On Windows, grab `atlas-x86_64-pc-windows-msvc.zip` from the [releases page](https://github.com/fkenmar/atlas/releases). Binaries for all platforms (x64 + arm64) are attached to every release by [cargo-dist](https://opensource.axo.dev/cargo-dist/).

**From source** — any platform, needs [Rust](https://rustup.rs):

```
git clone https://github.com/fkenmar/atlas
cd atlas
cargo install --path .
```

This builds `atlas` into `~/.cargo/bin` — make sure that's on your `PATH` (rustup sets this up by default).

Either way, verify and take it for a spin:

```
atlas --version
cd path/to/your/project
atlas .
```

You should see a `# atlas: …` header followed by a list of ranked files. That's the whole tool.

---

## Use

```
atlas .                          # map the current folder (2,048-token budget)
atlas . --budget 4096            # give it a bigger budget
atlas . --focus src/auth         # rank the files you're working on higher
atlas . --lang py,rs             # only these languages
atlas . --no-private             # public API surface only
atlas . --format json            # JSON instead of Markdown
```

By default atlas aims for a 2,048-token budget. When a repo doesn't fit, it degrades in steps rather than truncating: it drops private symbols (the header shows `public-only`), then elides parameter detail, then collapses the lowest-ranked files to a single line. Raise `--budget` for more detail, or use `--focus` to protect the paths you care about.

atlas caches parse results in a `.atlas/` directory at the repo root so re-runs are fast. It self-ignores (atlas writes `.atlas/.gitignore`), so it won't clutter your `git status`.

Pipe the output straight into your agent's context, or save it to a file:

```
atlas . > map.md
```

---

## Use it with your AI agent

atlas writes the map to stdout, so it drops into any agent's context.

**Save it and reference it** — works with Claude Code, Cursor, Windsurf, Copilot, or any chat:

```
atlas . > atlas-map.md
```

Then `@`-mention `atlas-map.md` in your prompt (or paste it in). Regenerate it whenever the structure changes — re-runs are warm-cached and finish in ~80 ms, so it's cheap to keep fresh.

**Pipe it inline** to any CLI agent:

```
{ echo "Repo map:"; atlas .; echo; echo "Task: add a --verbose flag"; } | your-agent
```

**Focus the budget on what you're touching** — `--focus` ranks those paths higher; repeat the flag for several:

```
atlas . --focus src/auth --focus src/api > atlas-map.md
```

**Keep it in the repo** so every contributor and agent starts oriented — commit `atlas-map.md` and regenerate it in a pre-commit hook or CI.

> An MCP server (`atlas serve --mcp`), so agents can pull a fresh map as a tool call, is on the roadmap.

---

## Why it works

Most of an agent's token bill on an unfamiliar repo is *exploration* — reading files to build a mental model. A map gives it that model up front, but a naive map (dumping every file) is too big and costs more than it saves. atlas earns its keep two ways:

1. **Structure only.** Signatures, types, and imports — never function bodies. That alone is a fraction of the source.
2. **Ranked and budgeted.** It scores every file by how central it is to the codebase and packs the most important ones into a fixed token budget, so the map stays small enough to live in context every turn.

```
discover → parse → link → rank → budget → render
```

It reads your repo with [tree-sitter](https://tree-sitter.github.io/tree-sitter/), respects `.gitignore` (and `.atlasignore`), and caches parse results so re-runs are fast.

---

## Troubleshooting

- **Empty map / "0 files".** atlas found no supported source under that path. Check the language is one it maps (Python, TS/JS, Rust) and that you're pointing at the project root — not a single file, and not a vendored or ignored directory.
- **`command not found: atlas`.** `~/.cargo/bin` isn't on your `PATH`. Add it (rustup's installer normally does), or run the binary by its full path.
- **A symbol is wrong or missing.** That's usually a tree-sitter extraction bug — please [open an issue](https://github.com/fkenmar/atlas/issues/new?template=bug_report.md) with a minimal snippet that reproduces it.

---

## Project status

Alpha. The core works end-to-end and is benchmark-tested, but the CLI and output format may still change. See [STATUS.md](STATUS.md) for the current state, [CHANGELOG.md](CHANGELOG.md) for what's landed, and [docs/PRD.md](docs/PRD.md) for the full design.

Coming next: an MCP server so agents can query the map directly (`atlas serve --mcp`), and an API-surface diff between revisions (`atlas diff HEAD~5`).

---

## Contributing

Conventions and workflow live in [CONTRIBUTING.md](CONTRIBUTING.md) and [CLAUDE.md](CLAUDE.md); architecture decisions in [docs/adr/](docs/adr/). To build and test:

```
cargo build && cargo test
```

<div align="center">
<sub>MIT © Kenmar</sub>
</div>
