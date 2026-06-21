# Claude Code MCP setup

atlas can run as a local MCP server over stdio so a compatible client can call
`get_map`, `get_symbol`, `get_symbol_index`, and `expand_symbol` instead of
relying on a committed map file.

Status: experimental but implemented. The server is read-only, makes no network
calls, and rejects paths outside its configured `--root`.

## Install atlas

Use one of the README install paths:

```sh
pipx install --pre atlas-map
```

or from a checkout:

```sh
cargo install --path .
```

Confirm the binary is on your path:

```sh
atlas --version
```

## Project-scoped config

Use this shape in a project-level MCP config file supported by your client:

```json
{
  "mcpServers": {
    "atlas": {
      "command": "atlas",
      "args": ["serve", "--mcp", "--root", "."]
    }
  }
}
```

The matching reusable example is
[examples/claude-code.mcp.json](../examples/claude-code.mcp.json).

If your client starts servers from a different working directory, replace `.`
with the absolute repository root.

## When to use MCP

Use MCP when the client can call tools during a session and you want a fresh map
on demand, especially after edits or when the user changes focus. Keep the plain
CLI path when you just need to paste or attach one map:

```sh
atlas . --for-agent -o atlas-map.md
```

Committing `atlas-map.md` can still be useful for repos where every contributor
should start with the same orientation. MCP is the integration layer; the CLI is
the stable core, and any future Claude/plugin packaging should wrap the CLI/MCP
surface rather than replace it.

## Tools

`get_map` arguments:

- `path`: repository root to map. Required, relative to `--root` or absolute
  under `--root`.
- `budget`: token budget. Optional; default is 2048.
- `format`: `md`, `json`, or `xml`. Optional; default is `md`.
- `no_private`: public API surface only. Optional.
- `lang`: extension filter such as `["py", "rs"]`. Optional.
- `focus`: paths to boost in the ranking. Optional.

`get_symbol` arguments:

- `path`: repository root to search. Required.
- `name`: exact symbol name. Required.
- `no_private`: public API surface only. Optional.
- `lang`: extension filter. Optional.

`get_symbol_index` arguments:

- `path`: repository root to index. Required.
- `budget`: token budget for the rendered anchor index. Optional; default is
  2048.
- `no_private`: public API surface only. Optional.
- `lang`: extension filter. Optional.

The result is a thin JSON index of stable anchors such as
`src/cache.rs#Cache` or `src/x.rs#from@42`; it intentionally omits signatures.

`expand_symbol` arguments:

- `path`: repository root the anchor is relative to. Required.
- `anchor`: `relpath#name` or `relpath#name@line`. Required.
- `no_private`: public API surface only. Optional.
- `lang`: extension filter. Optional.

The result returns the selected declaration's signature plus the defining file's
one-hop file-level imports and importers. It does not compute symbol-level
callers.

## Smoke test prompt

After adding the server to your client, ask:

```text
Use the atlas MCP server to get a 1200-token map for this repository, then tell
me which files define the CLI entry point and MCP tools. Do not open source
files until after you have read the map.
```

A healthy setup should call `get_map`, return Markdown by default, and mention
`src/cli.rs` plus `src/mcp.rs` before deeper source inspection.

## Suggested agent behavior

- Call `get_map` before broad repo exploration.
- Use `focus` when the user names a subsystem or file.
- Use `get_symbol` for "where is X defined?" before grepping.
- Use `get_symbol_index` when a full map would crowd the context window, then
  call `expand_symbol` for the few anchors that matter.
- Open the source before editing behavior; the map is an index, not the full
  implementation.

## Troubleshooting

- If a path is rejected, check that it is under the server's `--root`.
- If the map is empty, confirm the target uses supported extensions and is not
  excluded by `.gitignore` or `.atlasignore`.
- If the client cannot find `atlas`, use an absolute `command` path in the MCP
  config.
- If the server starts but tools are missing, restart the client after editing
  its MCP config; many clients read the config only at startup.
- If a request hangs, run `atlas . --budget 600` in the same directory. If the
  CLI fails there too, fix the repo/path issue before debugging MCP.
