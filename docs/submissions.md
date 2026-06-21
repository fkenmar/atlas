# Directory & awesome-list submissions

Ready-to-paste entries for getting atlas in front of the right audience.
Companion to [`post-launch-outreach.md`](post-launch-outreach.md) (the one-post-per-channel
plan). **Rules:** follow each list's CONTRIBUTING/PR template, disclose maintainer
affiliation, submit once, and never ask for stars. Lists drive a long tail of
discovery, so this is high-leverage and low-effort.

> Keep the wording here in sync with the README claim. The headline number is the
> **comprehension** benchmark (same accuracy, ~65% fewer tokens); never headline
> edit-task token deltas (too noisy — see `benchmark/README.md`).

## Repository metadata (set these in GitHub Settings first)

These aren't files — set them in **Settings → General** and the **About** panel,
since directories pull from them.

- **Description:**
  `Compile a codebase into a token-budgeted structural map for AI coding agents — same answers, ~65% fewer tokens. Rust CLI + MCP server.`
- **Homepage:** once GitHub Pages is enabled (below), set this to
  `https://fkenmar.github.io/atlas/`. Until then, the latest release URL is fine.
- **Topics:** `ai-agents`, `coding-agents`, `llm`, `mcp`, `claude`, `code-map`,
  `repo-map`, `tree-sitter`, `rust`, `cli`, `developer-tools`, `static-analysis`,
  `tokens`, `context-engineering`.

### Landing page (GitHub Pages)

A ready-to-serve landing page lives at [`index.html`](index.html) (with
`.nojekyll`). To publish it: **Settings → Pages → Source: Deploy from a branch →
`main` / `/docs`**. It comes live at `https://fkenmar.github.io/atlas/` — then set
that as the repo **Homepage** (above). The page leads with the reproducible
benchmark number and links back into the repo; share it as the one-click intro.

### Ready-to-open submissions

Paste-ready PR/forms for the first three targets are prepped in
[`submission-drafts.md`](submission-drafts.md) — open them in a couple of minutes
each.

## Reusable blurbs

**One-liner (≤100 chars):**

```
atlas — turn any repo into a ranked, token-budgeted structural map for AI coding agents.
```

**Short (awesome-list style):**

```
A fast Rust CLI (and MCP server) that compiles a codebase into a token-budgeted
structural map for AI coding agents — ranked signatures, types, and import edges,
no function bodies. In a comprehension benchmark, agents answered the same
questions with ~65% fewer tokens at identical accuracy.
```

**Medium (registry description):**

```
atlas gives an AI coding agent a navigation index before it starts opening files.
It extracts every signature, type, and import edge (never bodies), ranks files by
PageRank over the import graph, and packs the result into a token budget
(default ~2,048). Local, offline, deterministic. Markdown/JSON/XML output,
structural `diff`, and a read-only MCP stdio server. Languages: Python, TS/JS,
Rust, Go, Java, C/C++.
```

## Per-list entries

### awesome-mcp-servers (and similar MCP server lists)

Most use a bullet `- [name](url) - description.` under a category. atlas fits
**Developer Tools** / **Code Analysis**:

```markdown
- [atlas](https://github.com/fkenmar/atlas) 🦀 🏠 - Compiles a repo into a token-budgeted structural map (ranked signatures, types, imports — no bodies) for coding agents; exposes a read-only `get_map` tool over MCP stdio.
```

(Check the list's legend for emoji/badge conventions — 🦀 Rust, 🏠 local. Drop them
if the list doesn't use them.) MCP setup lives in
[`CLAUDE_CODE_MCP.md`](CLAUDE_CODE_MCP.md); the server is `atlas serve --mcp`.

### Official MCP registry (`modelcontextprotocol/registry`)

The registry takes a `server.json` published with the `mcp-publisher` CLI
(GitHub-auth proves you own `io.github.fkenmar/*`). Server facts, verified
against `src/mcp.rs`: server name `atlas`, transport **stdio**, tools
**`get_map`** and **`get_symbol`**, launched with `atlas serve --mcp --root .`.

Starting `server.json` (schema current as of 2025-12-11 — re-check the
`$schema` URL and validate with `mcp-publisher validate` before publishing):

```json
{
  "$schema": "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json",
  "name": "io.github.fkenmar/atlas",
  "title": "atlas",
  "description": "Token-budgeted structural map of a repo for AI coding agents — ranked signatures, types, and imports, no bodies. Read-only.",
  "repository": { "url": "https://github.com/fkenmar/atlas", "source": "github" },
  "version": "0.2.1-alpha",
  "packages": [
    {
      "registryType": "pypi",
      "registryBaseUrl": "https://pypi.org",
      "identifier": "atlas-map",
      "version": "PUT-EXACT-PYPI-VERSION-HERE",
      "transport": { "type": "stdio" },
      "runtimeHint": "uvx"
    }
  ]
}
```

**Before publishing, confirm two things** (the registry validates the package
exists and runs):

1. **Exact PyPI version string.** `0.2.1-alpha` is normalized by PyPI (likely
   `0.2.1a0`). Use the literal version shown on the
   [PyPI page](https://pypi.org/project/atlas-map/) for `packages[].version`.
2. **Entry-point launch.** The wheel installs the `atlas` command, not
   `atlas-map`, so a generic `uvx atlas-map` won't start the server. The real
   invocation is `uvx --from atlas-map atlas serve --mcp --root .`. Encode that
   with `packageArguments` (see the registry's server-json reference for the
   `--from`/subcommand pattern), or wait until atlas is on crates.io (#37) and
   publish a `registryType: "cargo"` package instead, which maps more cleanly to
   a single binary.

The local client config (`examples/claude-code.mcp.json`) already works today
regardless of registry status.

### MCP directories (Glama, Smithery, PulseMCP, etc.)

These index from the repo + a server manifest. Make sure the
[`examples/claude-code.mcp.json`](../examples/claude-code.mcp.json) config and the
MCP doc are current, then submit/claim the server with the *medium* blurb. They
typically auto-pull README and topics, so the metadata above does most of the
work.

### awesome-rust / awesome-cli-apps

Frame as a Rust CLI under **Development tools** / **Utilities**:

```markdown
- [atlas](https://github.com/fkenmar/atlas) - Compiles a codebase into a token-budgeted structural map (signatures, types, imports; no bodies) for AI coding agents. Ranked by PageRank, packed to a token budget, with a structural diff mode.
```

awesome-rust wants crates.io or active repos and alphabetical order — slot it
accordingly and follow its `CONTRIBUTING`.

### awesome-ai-coding / awesome-claude-code / LLM-tooling lists

Under **Context / repo understanding** or **Tools**:

```markdown
- [atlas](https://github.com/fkenmar/atlas) - Gives a coding agent a ranked, token-budgeted map of a repo so it stops burning context exploring. Works with any agent (stdout/file) or as an MCP server. ~65% fewer tokens at equal accuracy in a comprehension benchmark.
```

## Submission tracker

| Target | Format checked | Submitted | Merged | Notes |
| ------ | -------------- | --------- | ------ | ----- |
| awesome-mcp-servers | | | | |
| MCP registry | | | | |
| Glama / Smithery / PulseMCP | | | | |
| awesome-rust | | | | |
| awesome-cli-apps | | | | |
| awesome-ai-coding / awesome-claude-code | | | | |
