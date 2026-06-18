# ADR 0008 — MCP server over stdio with hand-dispatched JSON-RPC

## Context

Issue #7 (M2) adds an MCP server so an LLM agent can pull a fresh atlas map as a
tool call instead of being handed a static map — the core atlas-for-agents use
case (PRD §6, G4). The maintainer approved `serde_json`. Decisions needed: the
transport, how much JSON-RPC/MCP machinery to take on, and how the tool reuses
the map pipeline without disturbing the CLI.

## Decision

**Transport: stdio, newline-delimited JSON-RPC 2.0** — the MCP stdio transport.
`atlas serve --mcp` reads one JSON-RPC message per line from stdin and writes one
response per line to stdout (stderr stays free for logs). Routed git-style in
`run()` before `Cli::parse()` (the ADR 0005 pattern, same as `diff`), so the flat
map `Cli` is untouched.

**No framework, no derive — `serde_json::Value` + the `json!` macro.** Requests
are parsed as `Value` and dispatched on `method`; responses are built with
`json!`. This adds only `serde_json` (exactly what was approved), no `serde`
derive structs. A pure `handle(&Value) -> Option<Value>` function does the
dispatch (`None` for notifications), so the protocol is unit-testable without any
I/O; the stdin loop is a thin wrapper.

**Scope (v1):** `initialize`, `notifications/initialized` (no-op), `tools/list`,
`tools/call`, and `ping`. One tool, `get_map(path, budget?, no_private?,
format?)`, returns the rendered map as MCP text content. It calls a small
`build_map` helper local to the MCP module that runs discover → parse → link →
rank → budget (no `--focus`); the CLI's `run_with` pipeline is left as-is rather
than refactored under it, accepting a short, contained duplication to avoid any
risk to the well-exercised CLI path.

## Consequences

- Minimal dependency surface (one crate) and a testable protocol core.
- The map pipeline now appears in two places (`run_with` and `build_map`);
  unifying them behind one entry point (with typed errors so each caller formats
  its own messages) is a follow-up, deferred to keep this change low-risk.
- v1 exposes a single read-only tool; resources, prompts, `--focus`, and an
  incremental/watch-backed index are future additions (the watch daemon, #8,
  is the natural backing store).
- newline-delimited framing assumes messages contain no raw newlines — true for
  serialized JSON-RPC, which escapes them.
