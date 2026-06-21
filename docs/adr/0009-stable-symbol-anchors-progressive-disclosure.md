# ADR 0009 — Stable symbol anchors for progressive disclosure

## Context

The map already hits its token-efficiency ceiling: ADR 0004's symbol index gives
−65.2% tokens at 20/20 comprehension accuracy at the default 2,048 budget, and a
deep-research pass (2026-06-20) found the remaining per-turn cost is no longer the
map — a budgeted map placed in a prompt-cache-stable prefix re-reads at ~10% of
input price. The lever caching does *not* touch is **context-window occupancy**:
model recall degrades as the window fills, so emitting *less* up front still wins
even when re-reads are cheap. Anthropic's "just-in-time" context guidance and the
industry's "progressive disclosure" pattern both point the same way — front-load a
thin index of identifiers, load detail on demand.

atlas already has the pieces: `api::find_symbol` resolves declarations by
`(path, name)` into a `SymbolHit` (signature, kind, file, line, visibility), the
MCP server exposes `get_symbol`, and ADR 0004 already emits a compact ranked
symbol index for the collapsed tail. What's missing is a **stable way to address a
symbol** so the thin index can name a target and an agent can expand exactly that
target — without re-sending the whole map and without drifting into code
intelligence.

Decisions needed: the anchor format, how it resolves, and — critically — how much
an "expansion" is allowed to contain before it violates the structural/read-only,
not-an-LSP scope contract (PRD §3.2).

## Decision

**Anchor format: `relpath#name`.** A symbol is addressed by its repo-relative
file path and declared name, e.g. `src/cache.rs#Cache` or `src/auth/service.py#refresh_token`.
This reuses the exact key `find_symbol` already resolves on, adds no new persistent
state, and is human-readable so it works both in plain-Markdown maps and as an MCP
tool argument.

**Collision disambiguation: append `@<line>` only when needed.** Cross-file
duplicate names are already separated by the path prefix. Within one file, an
overloaded or repeated name (e.g. two `from` methods) disambiguates by appending
the 1-based declaration line: `src/x.rs#from@42`. The line is omitted in the
common (unique) case, so an anchor stays stable across edits that don't move the
declaration — important for prompt-cache stability and for diffs.

**Determinism (NFR-4).** Anchors are derived purely from `(relpath, name, line)`,
all of which are already deterministic. No content hashing (which would churn on
formatting changes), no ordinal IDs (which would churn on insertion). Same repo +
flags → byte-identical anchors.

**Expansion granularity — the scope boundary.** Resolving an anchor returns:
1. the symbol's full first-line **signature**, kind, and visibility (already in `SymbolHit`); and
2. its **defining file's one-hop file-level neighbors** — the importers (`used by`)
   and imports already in the file graph (ADR 0002).

It explicitly does **not** compute who *calls* the symbol at the symbol level. A
symbol-level call/reference graph requires name resolution across files — that is
go-to-definition / find-references, an LSP's job and a hard non-goal (PRD §3.2).
"Neighbors" stays file-granular, consistent with the map's existing `used by` /
`imports` lines. This keeps progressive disclosure firmly structural.

**Thin-index surface.** A render/MCP mode emits only the ranked anchor index
(`relpath#name`, type-first, no signatures), reusing ADR 0004's symbol-index
packing, so an agent front-loads a minimal navigable index and pulls detail via an
MCP `expand_symbol(path, name, line?)` tool (an extension of `get_symbol`) that
returns the signature + one-hop file neighbors for the anchor.

## Consequences

- The anchor is pure data with no new index to persist or invalidate; it resolves
  through the existing `find_symbol` path (read-only, cache-respecting).
- `@line` disambiguation only on collision keeps the common anchor stable across
  unrelated edits, preserving cache-stability and clean diffs (NFR-4).
- Expansion is one MCP round-trip per symbol — a latency/token trade-off that pays
  off only when the agent needs few symbols out of many; the thin index is opt-in,
  never the default full map, so repos that fit are unaffected.
- Holding the line at file-level neighbors keeps the feature out of LSP territory;
  a true call graph stays parked (ideas.md) as an explicit non-goal.
- Anchors are a stable public contract once shipped: their format is part of the
  MCP tool surface, so changing it later is a compatibility break (treated like the
  JSON schema — additive changes only where possible).
