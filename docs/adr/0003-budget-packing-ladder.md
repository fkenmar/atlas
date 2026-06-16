# ADR 0003 — Token-budget packing by exact render-and-count with a degradation ladder

## Context

The product thesis (PRD G1) is a map that fits a hard token budget (default
2,048) while keeping the highest-value structure and degrading gracefully. Two
coupled questions had to be settled together in the budget stage
(`src/budget.rs`):

1. **How to count tokens.** The budget must be enforced with *exact* BPE counts
   (FR-11), and the figure printed in the header ("rendered 1,991 tok") must be
   the number actually emitted. Estimating tokens per symbol and summing is
   cheap but wrong at the boundaries (BPE is not additive across joins) and
   would let the emitted map drift from the stated budget.

2. **What to drop when it doesn't fit, and in what order.** A 92k-LOC repo
   overflows 2,048 tokens by ~40×. Dropping whole low-rank files first
   (Aider-style) blanks out large-but-important files; worse, a single huge
   top-ranked file (pytest's `python.py`, 100+ symbols) can exceed the budget
   alone and, with a drop-or-keep rule, blank the *entire* map — observed
   directly: pytest rendered 0 content files. Naive ranking made it worse:
   scoring a file by the raw sum of its symbols' PageRank let symbol *count*
   dominate importance, so test files with 200 trivial functions outranked the
   core API.

Both failure modes were found by dogfooding the map on pytest before any agent
benchmark existed to measure them.

## Decision

The budget stage greedily packs by **rendering candidate maps and counting
their tokens exactly** — a `Tokenizer` trait over tiktoken-rs `cl100k_base`, so
`measure()` equals what `render()` emits. The header figure is self-referential
to within ~1 token, accepted because the budget is a target, not a hard cap
(PRD §5.3 shows 1,991 under 2,048).

Packing applies the PRD §5.1 ladder in a fixed order — **detail reduction
first, then file collapse.** Global rungs `Full → NoPrivate` (drop private
symbols) `→ NoParams` (strip parameter names, keep types) are each tried as a
complete listing; only if the most compact complete listing still overflows
does it greedily include files by rank. Each file is tried at its full block,
then — if that overflows the remaining budget — as a **one-line summary**
(`## path (#rank, N symbols)`); anything fitting neither collapses into a
directory-grouped footer that **always retains every un-shown file**, so the
directory skeleton is never lost.

Files are ranked for inclusion by **File-node PageRank plus the sum of each
symbol's rank *above* the uniform teleport baseline (1/N)** — earned importance,
not raw symbol score — which removes the symbol-count bias.

## Consequences

- **Budget and render are coupled by design:** `budget.rs` calls
  `render::markdown::render` to measure, and the greedy loop re-renders on each
  candidate (≈O(files-that-fit) renders, each bounded by the budget). Fine at 2k
  tokens / hundreds of files; a marginal-token-delta optimization is available
  later if profiling demands it.
- **The emitted map is exactly the measured map** — no estimator to keep in
  sync; the JSON `rendered` field and the markdown header always agree.
- The ladder's order encodes a value judgment — *more API at lower detail beats
  fewer files at full detail* — that may not be optimal for agents. The
  breadth/depth choice (one-line summaries vs. showing top-K symbols of a big
  file) is deliberately left as a **benchmark-tuned** refinement, not
  hard-coded conviction.
- Ranking by earned-rank-above-baseline is a heuristic, not a tuned weight; it
  killed the count bias, but the exact balance of import-rank vs. symbol-rank is
  itself a candidate for benchmark tuning.
- Determinism holds throughout: file order is `score desc` via `f64::total_cmp`
  then path, the footer groups via `BTreeMap`, and token counts are exact — same
  repo + flags ⇒ byte-identical map (NFR-4).
