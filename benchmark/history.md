# Benchmark history — append-only stats ledger

One row per measured change, appended by the self-improvement loop
(docs/SELF_IMPROVEMENT.md) and committed with the change it measures.
Medians, 3 runs/arm, claude-sonnet-4-6, max 30 turns, pytest 8.2.0, unless a
row says otherwise. "vs prev" compares to the most recent comparable row —
same arm, same metric version (M0 token metric: total input-side tokens
incl. cache; see benchmark/README.md).

| Date | Change measured | Arm | Task 01 tok/turns | Task 02 tok/turns | Pass | Δ vs prev |
|---|---|---|---|---|---|---|
| 2026-06-12 | no-map baseline recorded (M0 exit criterion) | without_map | 902,555 / 22 | 369,461 / 14 | 6/6 | — (baseline) |
| 2026-06-12 | naive-map probe — unofficial; ~81k-token unbudgeted map injected | with_map | 1,983,877 / 13 | 1,150,676 / 8 | 6/6 | turns −41% / −43%; tokens +120% / +211% (cache re-reads of oversized map = 92% of bill) |
