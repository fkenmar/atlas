---
name: atlas-orient
description: Use at the start of a coding task in an unfamiliar or large repository, before opening files — run `atlas` (or call the atlas MCP server) to get a ranked, token-budgeted structural map and orient. Also use when you've lost track of where things live, or need to find which file defines a symbol or which files depend on it.
---

# Orient with atlas before exploring

Before reading files to learn a repo's layout, get a structural map from **atlas** —
a ranked, token-budgeted index of every signature, type, and import edge (no
function bodies). It shows where things are in one shot, so you spend tokens on the
task instead of on exploration.

## Procedure

1. **Map first.** Run `atlas .` at the repo root (or read a committed `atlas-map.md`
   if one is present). Each file lists its public symbols, what it imports, and what
   depends on it, ranked by how central it is — `#1` is the most-depended-on file.
2. **Focus when you know the area.** `atlas . --focus src/auth --focus src/api`
   re-ranks the map toward the paths you're working in.
3. **Treat the map as an index, not source.** It carries signatures and structure
   only. Open the actual file before editing it or relying on an implementation
   detail.
4. **If the atlas MCP server is configured** (`atlas serve --mcp`), prefer its tools
   over blind grepping:
   - `get_map` — pull a fresh ranked map.
   - `get_symbol` — find where a name is defined (file, line, signature).
   - `get_symbol_index` then `expand_symbol` — pull a thin index of anchors, then
     expand only the symbols you actually need (progressive disclosure — keeps your
     context window small).

## Don'ts

- Don't grep file-by-file to learn the layout when a single map answers it.
- Don't treat the map as the source of truth for behavior — it's a navigation index;
  the source is.
- This skill is about *finding your way around the code*, never about how to write it.

Install atlas first so the `atlas` command is on PATH: `pipx install --pre atlas-map`
(or see https://github.com/fkenmar/atlas). atlas runs locally, read-only, offline.
