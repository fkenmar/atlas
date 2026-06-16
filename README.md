# atlas

> Turn any codebase into a compact map your AI coding agent can read in one shot — so it stops burning tokens exploring.

When an AI coding agent works in your repo, it spends most of its effort just figuring out where things are: opening file after file to learn the layout. **atlas** does that once and hands the agent a single ~2,000-token map — every function signature, type, and import, ranked by importance, with no function bodies. The agent gets its bearings immediately and gets to work.

In our benchmark, dropping an atlas map into the agent's context cut the tokens it spent exploring by **up to 78%** on "find the right code" tasks — with no loss of accuracy.

## What it looks like

Point atlas at a folder and it prints a map like this:

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

Files are ordered by importance (a PageRank over the import graph), `#1` being the most central. Each file shows its public symbols, what it imports, and what depends on it — everything an agent needs to navigate, nothing it doesn't. The whole thing is packed to fit your token budget.

## Install

From source (you'll need [Rust](https://rustup.rs)):

```
git clone https://github.com/fkenmar/atlas
cd atlas
cargo install --path .
```

This puts the `atlas` command on your PATH.

## Use

```
atlas .                          # map the current folder (2,048-token budget)
atlas . --budget 4096            # give it a bigger budget
atlas . --focus src/auth         # rank the files you're working on higher
atlas . --lang py,rs             # only these languages
atlas . --no-private             # public API surface only
atlas . --format json            # JSON instead of Markdown
```

Pipe the output straight into your agent's context, or save it to a file:

```
atlas . > map.md
```

**Languages:** Python, TypeScript/JavaScript, and Rust today. Go, Java, and C/C++ are planned.

## Why it works

Most of an agent's token bill on an unfamiliar repo is *exploration* — reading files to build a mental model. A map gives it that model up front, but a naive map (dumping every file) is too big and costs more than it saves. atlas earns its keep two ways:

1. **Structure only.** Signatures, types, and imports — never function bodies. That alone is a fraction of the source.
2. **Ranked and budgeted.** It scores every file by how central it is to the codebase and packs the most important ones into a fixed token budget, so the map stays small enough to live in context every turn.

```
discover → parse → link → rank → budget → render
```

It reads your repo with [tree-sitter](https://tree-sitter.github.io/tree-sitter/), respects `.gitignore` (and `.atlasignore`), and caches parse results so re-runs are fast.

## Project status

Alpha. The core works end-to-end and is benchmark-tested, but the CLI and output format may still change. See [STATUS.md](STATUS.md) for the current state, [CHANGELOG.md](CHANGELOG.md) for what's landed, and [docs/PRD.md](docs/PRD.md) for the full design.

Coming next: an MCP server so agents can query the map directly (`atlas serve --mcp`), and an API-surface diff between revisions (`atlas diff HEAD~5`).

## Contributing

Conventions and workflow live in [CLAUDE.md](CLAUDE.md); architecture decisions in [docs/adr/](docs/adr/). To build and test:

```
cargo build && cargo test
```

MIT © Kenmar
