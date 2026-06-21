# Ready-to-open submission packets

Three directory submissions, each prepared so you can open the PR (or form) in a
couple of minutes. Verify the target repo's current structure/section names
before submitting — lists reorganize — then paste. Follow each list's
`CONTRIBUTING`, disclose you're the maintainer, and never ask for stars.

Pre-flight (do once, first): set the repo **description + topics** from
[`submissions.md`](submissions.md) so the directories that auto-scrape look good.

---

## Packet 1 — awesome-mcp-servers

- **Target:** `punkpeye/awesome-mcp-servers` (the most-trafficked MCP list; if you
  prefer, `wong2/awesome-mcp-servers` uses the same pattern).
- **File:** `README.md`
- **Section:** find a fitting category — **Developer Tools** or
  **Code Analysis / Code Context**. Insert alphabetically within it.
- **Line to add:**

  ```markdown
  - [atlas](https://github.com/fkenmar/atlas) 🦀 🏠 - Compiles a repo into a token-budgeted structural map (ranked signatures, types, imports — no bodies) for coding agents; read-only map, symbol lookup, anchor-index, and anchor-expansion tools over MCP stdio.
  ```

  (Match the list's emoji legend; 🦀 = Rust, 🏠 = local/self-hosted. Drop them if
  the list doesn't use that convention.)
- **PR title:** `Add atlas (structural repo map for coding agents)`
- **PR body:**

  ```
  Adds atlas, a local Rust CLI + read-only MCP server that compiles a codebase
  into a token-budgeted structural map (signatures, types, imports; no function
  bodies) so a coding agent can navigate without reading every file.

  - Repo: https://github.com/fkenmar/atlas
  - MCP: `atlas serve --mcp` — tools `get_map`, `get_symbol`, `get_symbol_index`, `expand_symbol` (stdio, read-only)
  - Setup: https://github.com/fkenmar/atlas/blob/main/docs/CLAUDE_CODE_MCP.md

  Disclosure: I'm the maintainer. Happy to adjust the category or wording to fit
  the list's conventions.
  ```

---

## Packet 2 — MCP directories (Glama / Smithery / PulseMCP)

These mostly **auto-index from GitHub** or take a one-time claim/submit — usually
not a PR.

- **Glama** (`glama.ai/mcp/servers`): typically discovers public MCP repos
  automatically; sign in with GitHub and **claim** the atlas server, then confirm
  the description/category. Nothing to PR.
- **Smithery** (`smithery.ai`): submit/claim the server; if it wants a manifest,
  add a `smithery.yaml` describing the stdio start command
  (`atlas serve --mcp --root .`). Keep it in sync with
  [`examples/claude-code.mcp.json`](../examples/claude-code.mcp.json).
- **PulseMCP** (`pulsemcp.com`): has a "submit a server" form — paste the
  *medium* blurb from [`submissions.md`](submissions.md) and the repo URL.

**Blurb to paste (all three):**

```
atlas — a local Rust CLI and read-only MCP server that compiles a codebase into a
token-budgeted structural map for AI coding agents: ranked signatures, types, and
import edges, no function bodies. Tools: get_map, get_symbol, get_symbol_index,
expand_symbol (stdio). In a
comprehension benchmark, agents answered the same questions with ~65% fewer
tokens at identical accuracy.
```

---

## Packet 3 — awesome-rust (or an AI-coding list)

- **Target:** `rust-unofficial/awesome-rust` (broad reach). Alternative for a
  more on-topic audience: an `awesome-ai-coding` / `awesome-claude-code` list.
- **File:** `README.md`
- **Section:** **Applications → Development tools** (awesome-rust requires it be
  generally useful and points to an active repo / crates.io). Insert
  alphabetically.
- **Line to add:**

  ```markdown
  - [atlas](https://github.com/fkenmar/atlas) — Compiles a codebase into a token-budgeted structural map (signatures, types, imports; no bodies) for AI coding agents. Ranked by PageRank, packed to a token budget, with a structural `diff` mode.
  ```

- **PR title:** `Add atlas to Development tools`
- **PR body:**

  ```
  Adds atlas under Applications → Development tools.

  atlas is a single-binary Rust CLI that turns a repo into a ranked,
  token-budgeted structural map for AI coding agents (and an MCP server).
  Active, MIT-licensed, with CI and a benchmark suite.

  Repo: https://github.com/fkenmar/atlas

  Disclosure: I'm the maintainer.
  ```

  > awesome-rust prefers crates.io-published crates. If a reviewer pushes back,
  > that's a nudge to ship [#37 (publish to crates.io)](https://github.com/fkenmar/atlas/issues/37) —
  > or submit to an AI-coding list instead, where a binary/MCP tool fits cleanly.

---

## Tracker

Mirror the table in [`submissions.md`](submissions.md#submission-tracker) — mark
each Submitted/Merged as you go so you don't double-submit.
