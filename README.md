<div align="center">

<img src="assets/atlas-logo.svg" width="120" alt="atlas logo">

# atlas

**Turn any codebase into a compact map your AI coding agent can read in one shot — so it stops burning tokens exploring.**

<p>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-4F46E5?style=flat-square" alt="License: MIT"></a>
  <a href="https://github.com/fkenmar/atlas/releases"><img src="https://img.shields.io/badge/release-v0.2.1--alpha-4F46E5?style=flat-square" alt="Release"></a>
  <a href="https://github.com/fkenmar/atlas/actions/workflows/ci.yml"><img src="https://github.com/fkenmar/atlas/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/built%20with-Rust-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Built with Rust"></a>
  <img src="https://img.shields.io/badge/languages-Py%20%C2%B7%20TS%20%C2%B7%20Rust%20%C2%B7%20Go%20%C2%B7%20Java%20%C2%B7%20C%2FC%2B%2B-F59E0B?style=flat-square" alt="Languages: Py · TS · Rust · Go · Java · C/C++">
</p>

<sub>
  <a href="#install"><b>Install</b></a> ·
  <a href="#60-second-quickstart"><b>Quickstart</b></a> ·
  <a href="#use"><b>Use</b></a> ·
  <a href="#use-it-with-your-ai-agent"><b>Use it with your agent</b></a> ·
  <a href="#docs-for-agents"><b>Docs for agents</b></a> ·
  <a href="#why-it-works"><b>Why it works</b></a> ·
  <a href="#troubleshooting"><b>Troubleshooting</b></a> ·
  <a href="docs/FAQ.md"><b>FAQ</b></a>
</sub>

<br>
<br>

<img src="assets/atlas-demo.gif" width="820" alt="atlas demo: map a whole repo, focus a file to rerank it, and save the map for your agent">

</div>

---

When an AI coding agent works in your repo, it spends most of its effort just figuring out where things are: opening file after file to learn the layout. **atlas** does that once and hands the agent a single ~2,000-token map — every function signature, type, and import, ranked by importance, with no function bodies. The agent gets its bearings immediately and gets to work. It's the idea behind aider's repo map, unbundled into a standalone tool you can point at any agent.

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

Want to see more output before installing? The [example gallery](examples/gallery/) has real maps for a Python service, a TypeScript app, and a mixed Go/Rust/Python repo, each with the exact command used.

**What it maps:** Python (`.py`, `.pyi`), TypeScript/JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, …), Rust (`.rs`), Go (`.go`), Java (`.java`), and C/C++ (`.c`, `.h`, `.cpp`, `.hpp`, …). It honors your `.gitignore` and an optional `.atlasignore`, and always skips hidden directories and common vendored/build folders (`node_modules`, `target`, `dist`, `build`, `venv`, `__pycache__`, `vendor`, …). If a file you expected isn't there, it's almost always an unsupported language or a skipped directory. For exactly what's extracted per language — symbol kinds, visibility rules, import-linking behavior, and caveats — see the [language support matrix](docs/languages.md).

---

## Install

**pip / pipx** — for the Python crowd, no Rust required:

```
pipx install --pre atlas-map     # or: pip install --pre atlas-map
```

atlas is a Rust binary, not a Python package — the wheel just drops the native `atlas` command onto your PATH (the same way `ruff` and `uv` ship). The PyPI distribution is named `atlas-map` because `atlas` was taken; the command you run is still `atlas`. While atlas is in alpha the `--pre` flag is required.

**Prebuilt binary** — no Rust required (macOS & Linux):

```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fkenmar/atlas/releases/download/v0.2.1-alpha/atlas-installer.sh | sh
```

On Windows, grab `atlas-x86_64-pc-windows-msvc.zip` from the [releases page](https://github.com/fkenmar/atlas/releases). Binaries for all platforms (x64 + arm64) are attached to every release by [cargo-dist](https://opensource.axo.dev/cargo-dist/). For Windows-specific `PATH`, PowerShell, and completion setup, see the [Windows guide](docs/windows.md).

**From source** — any platform, needs [Rust](https://rustup.rs):

```
git clone https://github.com/fkenmar/atlas
cd atlas
cargo install --path .
```

This builds `atlas` into `~/.cargo/bin` — make sure that's on your `PATH` (rustup sets this up by default).

Behind a corporate proxy, on an air-gapped host, or installing from an internal mirror? See [`docs/install-offline.md`](docs/install-offline.md). atlas runs fully offline after install.

Either way, verify and take it for a spin:

```
atlas --version
cd path/to/your/project
atlas .
```

You should see a `# atlas: …` header followed by a list of ranked files. That's the whole tool.

atlas runs locally and does not collect telemetry; see
[`docs/PRIVACY.md`](docs/PRIVACY.md) for the offline/privacy model.

---

## 60-second quickstart

From zero to a map your agent can read:

```
pipx install --pre atlas-map     # or any install method above
atlas --version                  # confirm it's on your PATH
cd path/to/your/project
atlas . --budget 1024            # a quick, small map to stdout
atlas . -o atlas-map.md          # save the full default map to a file
```

Success looks like a header and a ranked file list — the most central file is
`#1`:

```
# atlas: your-project (3740 LOC, 16 files) | budget 1024 | rendered 1012 tok

## cache.rs (#1 — imported by 1 file(s))
pub struct Cache
    pub fn open(&Path) -> Cache
...
```

If you instead see **`no supported source files found`**, it's almost always the
two first-run gotchas: you're pointing at a single file or a subfolder instead of
the **repo root**, or the project is in a language atlas doesn't map yet (the
error lists the file extensions it saw). Head to the full [usage](#use) and
[troubleshooting](#troubleshooting) sections from here.

---

## Use

```
atlas .                          # map the current folder (2,048-token budget)
atlas . --budget 4096            # give it a bigger budget
atlas . --focus src/auth         # rank the files you're working on higher
atlas . --lang py,rs             # only these languages
atlas . --no-private             # public API surface only
atlas . --format json            # JSON instead of Markdown
atlas . --format xml             # XML, for wrapping in a Claude prompt
atlas . -o atlas-map.md          # atomically write to a file
atlas . --for-agent              # add a short Markdown note for agent context
atlas . --timings                # print stage timings to stderr
atlas diff HEAD~1 HEAD           # structural delta between two git revisions (or two dirs)
atlas serve --mcp                # experimental MCP stdio server
atlas . --color always           # force ANSI color (auto-detects a terminal otherwise)
```

When you run atlas in a terminal the Markdown map is colorized for scannability; piped, redirected, or `--output` file output stays plain, so feeding it to an agent or a file is unaffected (`--color never` to disable, `NO_COLOR` is honored).

**Shell completions:** `atlas --completions <bash|zsh|fish|powershell|elvish>` prints a completion script — e.g. `atlas --completions zsh > ~/.zfunc/_atlas`.

**Pasting maps into an agent?** A map is untrusted data derived from source. `--format xml` escapes the content so it can't break out of its container — see the [prompt-injection threat model](docs/prompt-injection.md) for safe wrappers and what escaping does (and doesn't) guarantee.

By default atlas aims for a 2,048-token budget. When a repo doesn't fit, it degrades in steps rather than truncating: it drops private symbols (the header shows `public-only`), then elides parameter detail, then collapses the lowest-ranked files to a single line. Raise `--budget` for more detail, or use `--focus` to protect the paths you care about.

atlas caches parse results in a `.atlas/` directory at the repo root so re-runs are fast. It self-ignores (atlas writes `.atlas/.gitignore`), so it won't clutter your `git status`.

Pipe the output straight into your agent's context, or save it to a file:

```
atlas . -o map.md
```

`--output` writes through a same-directory temporary file and then renames it into place, so a failed run does not leave a partial map at the final path.

---

## Use it with your AI agent

atlas writes the map to stdout, so it drops into any agent's context.

**Save it and reference it** — works with Claude Code, Cursor, Windsurf, Copilot, or any chat:

```
atlas . -o atlas-map.md
```

Then `@`-mention `atlas-map.md` in your prompt (or paste it in). Regenerate it whenever the structure changes — re-runs are warm-cached and finish in ~80 ms, so it's cheap to keep fresh.

For a pasted or attached Markdown map, `--for-agent` prepends a short note telling the agent to treat the map as a navigation index, not as source:

```
atlas . --for-agent -o atlas-map.md
```

**Pipe it inline** to any CLI agent:

```
{ echo "Repo map:"; atlas .; echo; echo "Task: add a --verbose flag"; } | your-agent
```

**Focus the budget on what you're touching** — `--focus` ranks those paths higher; repeat the flag for several:

```
atlas . --focus src/auth --focus src/api > atlas-map.md
```

**Keep it in the repo** so every contributor and agent starts oriented — commit `atlas-map.md` and regenerate it in a [pre-commit hook](docs/pre-commit.md) or [CI](docs/ci-recipes.md), or point your [`CLAUDE.md` / `AGENTS.md`](docs/agent-files.md) at it.

For copy-paste recipes per agent (Claude Code, Cursor, Copilot, terminal agents) and per-editor [task snippets](docs/editor-snippets.md), see the [agent integration cookbook](docs/agent-cookbook.md).

> Experimental on `dev`: `atlas serve --mcp` exposes a `get_map` tool over stdio JSON-RPC/MCP so compatible agents can pull a fresh map directly.

---

## Docs for agents

Agent documentation entry points live in [`llms.txt`](llms.txt) and
[`llms-full.txt`](llms-full.txt). They point agents at the README, benchmark
history, [comparison guide](docs/comparison.md), [security policy](SECURITY.md),
PRD, changelog, and MCP setup docs without turning the README into a doc index.

For Claude Code and other MCP-compatible clients, see
[`docs/CLAUDE_CODE_MCP.md`](docs/CLAUDE_CODE_MCP.md) and the reusable
[`examples/claude-code.mcp.json`](examples/claude-code.mcp.json) config.

---

## Why it works

Most of an agent's token bill on an unfamiliar repo is *exploration* — reading files to build a mental model. A map gives it that model up front, but a naive map (dumping every file) is too big and costs more than it saves. atlas earns its keep two ways:

1. **Structure only.** Signatures, types, and imports — never function bodies. That alone is a fraction of the source.
2. **Ranked and budgeted.** It scores every file by how central it is to the codebase and packs the most important ones into a fixed token budget, so the map stays small enough to live in context every turn.

```
discover → parse → link → rank → budget → render
```

It reads your repo with [tree-sitter](https://tree-sitter.github.io/tree-sitter/), respects `.gitignore` (and `.atlasignore`), and caches parse results so re-runs are fast.

For a factual comparison against Aider repo-map, ctags, tree-sitter CLI,
Sourcegraph/SCIP, and concat-style repo packers, see
[`docs/comparison.md`](docs/comparison.md).

---

## Troubleshooting

- **Empty map / "0 files".** atlas found no supported source under that path. Check the language is one it maps (Python, TS/JS, Rust, Go, Java, C/C++) and that you're pointing at the project root — not a single file, and not a vendored or ignored directory. The error includes the top file extensions atlas saw to make wrong-root or unsupported-language cases easier to spot.
- **`command not found: atlas`.** `~/.cargo/bin` isn't on your `PATH`. Add it (rustup's installer normally does), or run the binary by its full path.
- **A symbol is wrong or missing.** That's usually a tree-sitter extraction bug — please [open an issue](https://github.com/fkenmar/atlas/issues/new?template=extraction_bug.md) with a minimal snippet that reproduces it.
- **Too much noise, or a missing file in a big repo.** Tune what atlas maps with `.atlasignore`, `--focus`, `--lang`, or by mapping a subdirectory — see [`docs/monorepos.md`](docs/monorepos.md).
- **Scripting atlas in CI.** The exit codes are a stable contract (`0` success, `1` operational error, `2` usage error) — see [`docs/exit-codes.md`](docs/exit-codes.md).
- **Old `repomap` names or files** (`.repomapignore`, `.repomap/`, `REPOMAP.md`). atlas doesn't read them — see the [migration guide](docs/MIGRATION.md).
- **On Windows** — `PATH`, PowerShell, execution policy, and completions are covered in the [Windows guide](docs/windows.md).

More answers in the [FAQ](docs/FAQ.md).

---

## Project status

Alpha. The core works end-to-end and is benchmark-tested, but the CLI and output format may still change — pin a version if you depend on the output. See [STATUS.md](STATUS.md) for the current state, [CHANGELOG.md](CHANGELOG.md) for what's landed, and [docs/PRD.md](docs/PRD.md) for the full design.

`atlas diff <old> <new>` shows the structural delta between two trees — added/removed/changed signatures and import edges — so an agent sees what moved without re-reading the tree. Each side is a directory **or a git revision** (`atlas diff HEAD~1 HEAD`, `atlas diff v0.2.0 .`); revisions are checked out via `git` under the hood (no extra setup). Markdown by default; `--format json` or `xml` for tooling and CI.

Experimental on `dev`: an MCP stdio server (`atlas serve --mcp`) lets compatible agents query the map directly through a `get_map` tool.

---

## Contributing

Conventions and workflow live in [CONTRIBUTING.md](CONTRIBUTING.md) and [CLAUDE.md](CLAUDE.md); architecture decisions in [docs/adr/](docs/adr/). To build and test:

```
cargo build && cargo test
```

<div align="center">
<sub>MIT © Kenmar</sub>
</div>
