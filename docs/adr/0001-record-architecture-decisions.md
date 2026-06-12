# ADR 0001 — Record architecture decisions

## Context

repomap is built by a single maintainer at ~10 hours/week, with long gaps between work sessions and heavy agent-assisted development. Design reasoning evaporates across those gaps: two weeks later, neither the maintainer nor a fresh Claude session remembers why a choice was made, and decisions get accidentally relitigated or silently reversed. Git history records *what* changed, never the alternatives that were weighed and rejected.

## Decision

We record significant architecture decisions as Architecture Decision Records in `docs/adr/`, one file per decision, numbered sequentially (`NNNN-slug.md`), using exactly this structure: **Context** (the forces in play), **Decision** (what we chose, in full sentences, specific enough to act on), **Consequences** (what becomes easier, harder, or constrained). The bar for "significant": anything whose reasoning you would forget after two weeks away. ADRs are one page maximum and append-only — a decision is amended or reversed by a new ADR that names and supersedes the old one, never by editing the original. The `/adr` slash command and the adr-writing skill produce them.

## Consequences

- Every session — human or agent — can recover the "why" behind the architecture from `docs/adr/` without archaeology in git history; the SessionStart hook surfaces the latest titles automatically.
- Writing a page per decision adds friction; the one-page maximum and the two-week-memory bar keep the cost proportional.
- Because ADRs are immutable, `docs/adr/` is edit-protected by a PreToolUse hook for already-numbered files; superseding a decision costs a new number, which keeps the historical record honest.
