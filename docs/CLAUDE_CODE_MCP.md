# Claude Code MCP setup

atlas can run as a local MCP server over stdio so a compatible client can call
`get_map` and `get_symbol` instead of relying on a committed map file.

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

## Suggested agent behavior

- Call `get_map` before broad repo exploration.
- Use `focus` when the user names a subsystem or file.
- Use `get_symbol` for "where is X defined?" before grepping.
- Open the source before editing behavior; the map is an index, not the full
  implementation.

## Troubleshooting

- If a path is rejected, check that it is under the server's `--root`.
- If the map is empty, confirm the target uses supported extensions and is not
  excluded by `.gitignore` or `.atlasignore`.
- If the client cannot find `atlas`, use an absolute `command` path in the MCP
  config.
