# ADR 0004 — Symbol index for the collapsed tail

## Context

At a tight budget (the default 2,048 tokens) on a large repo, the degradation
ladder (ADR 0003) collapses low-rank files into a bare directory skeleton —
`[259 low-rank file(s) collapsed: src/* (65), testing/* (158)]`. That line
records *that* files exist but erases *what* they define: every class, type, and
function in the tail becomes invisible.

The comprehension benchmark measured the cost. Only **8 of 20** structural
answers ("which class implements X, and where") were present in the default
map; for the other twelve the agent fell back to grepping — 2–3 extra turns at
~85k tokens each. An A/B smoke pinned the mechanism precisely: when the answer
is already in the map the agent replies in **1 turn (~30k tokens)**; when it
isn't, **3 turns (~90k)**. Answer-in-map is the lever, and the collapsed tail
was throwing answers away.

Raising the budget surfaces more answers but bloats *every* question's prompt
(including the easy ones), so it's the wrong knob. The collapsed files' symbol
*names* are what's missing, and a name costs ~6 tokens against ~20 for a full
signature — cheap enough to list many.

## Decision

In rung 3 (the only rung where files collapse), reserve a fraction of the budget
(`INDEX_RESERVE_FRACTION = 0.40`) for a **compact symbol index**: bare
`path: NameA, NameB, …` lines naming the navigable declarations of files not
shown in full (collapsed files, one-line files, and the omitted symbols of
partial files).

- **Type-first.** Classes, interfaces, enums, and type aliases are listed across
  the whole rank order *before* any function or constant, so a rank-43 core
  class lands before rank-2's helper functions. Types are what an agent
  navigates *to*; functions are reached from within.
- **Capped per file** (`INDEX_TYPES_PER_FILE = 8`, `INDEX_FUNCS_PER_FILE = 2`,
  kept by symbol PageRank) so a symbol-heavy top file can't monopolize the index
  and starve the long tail of coverage.
- **Greedily fit** by binary search over the rank-ordered candidate list (token
  count is monotonic in prefix length), so the index fills exactly to the
  budget.

It is purely additive: the directory skeleton, the file blocks, and every other
behavior are unchanged, and the index only appears when files collapse — repos
that fit in full never see it. The reserve fraction and caps are
benchmark-tunable constants.

## Consequences

- **Coverage at the default 2,048 budget rose 8 → 12 of 20** structural answers,
  flipping the *median* benchmark question from a grep to a one-turn answer.
- **Validated end-to-end** (comprehension 20-question A/B, run-20260617-084740):
  **−65.2% median tokens** (85,670 → 29,781), **3 → 1 median turns**, at
  **20/20 accuracy in both arms** — zero accuracy regression from the terser
  map. This more than doubles the pre-index result (−30.1%) at the same budget,
  and lands at the harness ceiling (~65%; the ~28–30k/turn fixed overhead of the
  agent runtime makes ~70% unreachable from the map side alone).
- **Cost:** at a tight budget, marginal files now show ~40% less full-signature
  detail (traded for name→file breadth across the whole repo). For navigation
  and structural Q&A this is a strict win; a workflow that needs full signatures
  of mid-rank files should raise `--budget`.
- **Known limitation:** a symbol with low PageRank but high human salience (e.g.
  `CaptureManager`, invoked via hooks rather than direct calls) can miss its
  file's per-file cap, and files below ~rank 40 miss at 2,048 (both clear at
  3,072). Better per-symbol ranking would lift these; that's a separate change.
- The JSON schema gains a `symbol_index` array (additive; `SCHEMA_VERSION`
  unchanged since no existing field moved).
