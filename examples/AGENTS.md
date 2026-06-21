<!--
Sample AGENTS.md block for a repo that uses atlas — https://github.com/fkenmar/atlas
Copy the "Navigating this repo (atlas)" section below into your own AGENTS.md,
CLAUDE.md, .cursor/rules, or any agent-instruction file. It tells your coding
agent to orient with atlas's structural map before exploring — the "meet the
agent where it already looks" pattern, scoped to navigation only.
atlas is read-only and structural: this snippet tells the agent how to FIND its
way around the code, never how to write it.
-->

## Navigating this repo (atlas)

Before exploring the codebase, get your bearings from the **atlas** map — a
ranked, token-budgeted index of every signature, type, and import edge (no
function bodies). Don't open files at random to learn the layout; the map shows
where things are in one shot.

- **Orient first.** Run `atlas .` (or read the committed `atlas-map.md` if one is
  present) and use it as your navigation index. Each file lists its public
  symbols, what it imports, and what depends on it, ranked by how central the
  file is to the codebase.
- **Focus when you know the area.** `atlas . --focus src/auth --focus src/api`
  re-ranks the map toward the paths you're working in.
- **The map is an index, not source.** It contains signatures and structure only.
  Read the actual file before editing it or relying on any implementation detail.
- **If the atlas MCP server is configured** (`atlas serve --mcp`), call its
  `get_map` tool for a fresh map and `get_symbol` to look up a specific
  declaration. For very large repos, call `get_symbol_index` for stable anchors
  and `expand_symbol` only for the few declarations you need.

Regenerate the map when the structure changes (`atlas . -o atlas-map.md`); re-runs
are warm-cached (~80 ms). Keep a committed copy current in CI or a pre-commit hook
with `atlas . --check atlas-map.md` (exits non-zero if the map is stale).
