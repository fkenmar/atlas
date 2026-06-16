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
| 2026-06-16 | **M1 budgeted map — first fair with_map checkpoint** (~2,042-tok map; N=2 PRELIMINARY, below the 3-run protocol) | with_map | 614,963 / 15 | 459,756 / 15 | 4/4 edit | edit-token Δ vs baseline: −31.9% (t01) / +24.4% (t02), aggregate **−15.5% → FAIL ≥25%**. BUT same-run with_map vs without_map = **−26.0% → PASS**. Variance >15% throughout (t02 without_map 84% spread, turn-cap blowout); baseline without_map (369k) did NOT reproduce (793k today). Spend $6.11. |

**Comprehension checkpoint (2026-06-16, 6 questions, read-only):** without_map 6/6 acc, 54,300 tok / 2 turns; with_map **6/6 acc, 29,668 tok / 1 turn**. Hard gate (with_map acc ≥ without_map): **PASS** — equal accuracy, ~45% fewer tokens, half the turns, 4/6 resolved in a single turn. The map is an unambiguous understanding win.

**Read of the checkpoint:** the budgeted map clearly helps (comprehension), and helps most where structure matters (task 01, the multi-site edit, −31.9%). The ≥25% edit verdict is **uncertain at N=2** and limited by the *protocol/metric*, not the map: the token proxy (`cache_read` accumulating with turn count) lets any run that hits the 30-turn cap dominate the median, and `baseline.json`'s without_map arm is not reproducing on the current CLI. Next protocol moves (some are human gates): re-record the without_map baseline (`run.sh --record-baseline`), run N≥3 with an odd count for a true median, refine the exploration-token metric toward the PRD definition (tokens before first correct edit), and replace task 02 with a more knowledge-sensitive task. See benchmark/results/run-20260616-013153.local.json + comprehension-20260616-012947.local.json.

---

| 2026-06-16 | **REFINED metric + density-improved map** (results schema 3: exploration tokens before first edit, medians over passing+non-capped runs; cap 45; same-run with_map vs without_map, N=3) | with_map vs without_map | t01: 1,210,914 vs 422,415 = **−186.7%** | t02: 920,401 vs 4,195,977 = **+78.1%** | 12/12 pass, 0 capped | aggregate **+53.9%** (clears ≥25%, ~26pt below 80%) |

**The cleanest measurement so far** (run-20260616-101017): the new metric + cap-exclusion + raised cap eliminated capping (0/12 vs 6/12 in the cap-30 run-091804) and every run passed. Two density commits (adaptive footer + resolved internal imports) freed ~50% of the budget for real API. Result is **task-type-dependent**, not uniform:
- **Task 02 (locate-a-utility): +78.1% exploration reduction** — within 2 points of the 80% goal, and the only arm with near-acceptable variance (19%). The map surfaces the `absolutepath` helper so the agent skips the tree walk (turns 47→15, assistant-msgs-to-edit 80→24). This is the density wins paying off: the same task read +24% WORSE on the noisy cap-30 run.
- **Task 01 (multi-site edit): −186.7%** — the map made the agent over-explore on 2 of 3 runs (1.2M tok, ~43 turns) though one run was fast (279k, beats without_map). Bimodal, 84% variance.
- **Pattern (handed back, not yet acted on):** the map helps "find the existing thing" tasks and may mislead on "find all the places to change" (multi-site) tasks — which need reverse-reference info (who uses a symbol) the map doesn't surface yet.
- **Trust:** variance is still 62–84% on 3 of 4 arms with only 3 clean runs — per-task numbers (esp. task 01) need **N≥5** to be stable. Denied-Bash thrashing (task-02 without_map ran 47–50 turns) is a candidate cause; relaxing it is a symmetric protocol change to consider.

See benchmark/results/run-20260616-101017.local.json.
