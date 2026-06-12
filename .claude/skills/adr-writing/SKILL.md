---
name: adr-writing
description: Use when making or recording an architecture decision — choosing between designs, libraries, data structures, or output formats; when /adr is invoked; or when amending or reversing a past decision recorded in docs/adr/.
---

# Writing ADRs

## What rises to ADR level

The test: **would you forget the reasoning after two weeks away?** This project runs at ~10 hrs/week with long gaps — anything that passes that test gets an ADR. Typical: dependency choices, data-structure strategies (ADR 0002 is the canonical example), output-format contracts, evaluated-and-rejected alternatives (the rejection reasoning is the valuable part). Not ADR-level: naming, formatting, anything clippy or rustfmt decides, reversible one-liners.

## Template

```markdown
# ADR NNNN — <Title: the decision as a noun phrase or imperative>

## Context

The forces in play: requirements, constraints, what was tried or considered
and why it lost. Written so a reader with no session context understands
why a decision was needed at all.

## Decision

What we chose, in full sentences, specific enough to act on. "We use X"
not "we should consider X".

## Consequences

What becomes easier, harder, or constrained — honest about the costs, not
a sales pitch. Include the mitigation if a known weakness has one.
```

## Rules

- One file per decision: `docs/adr/NNNN-slug.md`, where NNNN is the next number (4 digits, zero-padded — check `ls docs/adr/` for the highest existing).
- **One page maximum.** If it needs more, the decision isn't crisp yet.
- **Append-only**, enforced by a PreToolUse hook: never edit an existing ADR. To amend or reverse one, write a NEW ADR that names the one it supersedes ("Supersedes ADR 0003").
- Number and title go in the H1; the SessionStart hook surfaces the last three titles, so make titles self-explanatory.
