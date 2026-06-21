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

---

| 2026-06-17 | **N=5 confirmation + reverse-ref/field lever** (first run measuring the current map: `used by` edges + class fields — commits a42c7f9/144051b/d29a67d/0c846db; schema 3, exploration tokens, medians over passing non-capped, cap 45) | with_map vs without_map | t01: 651,823 vs 390,744 = **−66.8%** | t02: 1,638,447 vs 2,123,785 = **+22.9%** | 20/20 pass (1 t02 with_map capped) | aggregate **+8.9%** (sum of medians) — **FAILS the ≥25% bar**; variance 64–139% |

**The decisive N=5 (run-20260616-232455, spend $14.12).** First measurement of the current map (reverse-dependency "used by" edges + class fields). Verdict: **no clear token win, and nowhere near the 80% goal.**
- **Variance did NOT collapse at N=5.** without_map spreads are **139%** (t01) and **127%** (t02) of the median; the agent's run-to-run nondeterminism swings exploration 3–10× (t02 without_map ranged 784k → 3.5M). The medians are still soft — even N=5 doesn't pin this down.
- **Aggregate +8.9%** (sum of medians) — below the ≥25% "measurable win" bar. The earlier N=3 **+53.9% / +78%** was largely an artifact of one run's without_map blowup (t02 without_map was 4.2M in run-101017 vs 2.1M here); it did not hold up.
- **The robust signal is the task split:** the map helps "find the existing thing" (t02 **+22.9%**) and hurts "find all the sites" multi-site edits (t01 **−66.8%**).
- **The reverse-ref/field lever helped t01** (with_map t01 went 1.21M @ −186.7% in run-101017 → 652k @ −66.8% here) but did **not** flip it positive — keep the features, but the denser map still costs more than it saves on multi-site, token-wise.
- **Turns improve with the map in both tasks** (t01 22→17, t02 24→17, ≈ **−25%**): the map makes the agent more *turn*-efficient even where it isn't *token*-efficient.
- **Bottom line for M1:** the "measurable benchmark win" exit criterion is **not met on edit-task tokens.** The defensible wins are **turns (−25%)** and **comprehension (−45% tokens at equal accuracy, earlier)**. The 80% token-reduction goal is out of reach with this approach. The benchmark's dominant problem is variance, not the map — stabilizing it (task redesign, trimmed means, larger N) is the prerequisite for any trustworthy token verdict.

See benchmark/results/run-20260616-232455.local.json.

---

**Comprehension worthiness gate — PASSED (2026-06-17, run-004810, 20 verified questions, sonnet, read-only both arms).** The trustworthy, low-variance signal atlas can stand behind:
- **Accuracy: 20/20 both arms (100%)** — equal, perfect, **zero per-question regressions** (the map never made the agent wronger). Hard gate (with_map ≥ without_map): **PASS**.
- **Tokens: 85,817 → 59,916 = −30.1%** at identical accuracy. **Turns: 3 → 2 (−33%).**
- This is the download-worthy claim: the map lets an agent locate structural elements with ~30% fewer tokens and a third fewer turns without losing any accuracy. Unlike edit-task token deltas (60–140% variance), comprehension is constrained and stable across 20 questions — this is the number that goes in the README, and the basis for moving to usability/ship work. See benchmark/results/run-20260617-004810.local.json.

---

**Symbol index — collapsed tail made navigable (2026-06-17, run-20260617-084740, ADR 0004).** The biggest comprehension win yet, and the answer to "70% fewer tokens at identical accuracy."
- **Tokens: 85,670 → 29,781 = −65.2%** at the shipped default 2,048 budget. **Turns: 3 → 1.** **Accuracy: 20/20 both arms** — zero regression from the terser map (the hard gate holds).
- **More than doubles the prior −30.1%** at the *same* budget. The lever: a free A/B smoke proved answer-in-map ⇒ 1-turn answer (~30k tok) vs grep ⇒ 3 turns (~90k). The old footer collapsed the long tail to a bare directory skeleton (`src/* (65)`), erasing every symbol; only 8/20 answers were in the default map.
- **The fix (ADR 0004):** reserve 40% of the budget in rung 3 for a compact `path: TypeA, TypeB` index of the collapsed/degraded files' navigable declarations — type-first, ranked, capped per file (8 types / 2 funcs), greedily fit by binary search. Purely additive; off when a repo fits in full.
- **Free coverage proof (in-map answer presence, the proxy for the token win):** default 2,048 went **8 → 12/20**; 3,072 went ~10 → **17/20**. Crossing 10/20 flips the *median* question to a one-turn answer — which is exactly what the median-token metric rewarded.
- **Ceiling reality:** −65% is the harness floor. The agent runtime's ~28–30k/turn fixed overhead caps map-side reduction near (85.7−30)/85.7 ≈ 65%; ~70% is unreachable without shrinking that overhead, which atlas can't touch. This is the goal, met as far as physics allows.
- **Known gaps:** `CaptureManager` (low PageRank in a 17-class file) and `CallInfo` (rank 43, past 2,048 index depth) still miss at the default budget; both clear at 3,072. Better per-symbol ranking is the next lever. See benchmark/results/comprehension-20260617-084740.local.json.

**Regression caught + fixed — anchor-render bloat (2026-06-21).** Re-running the comprehension benchmark on current `main` (post progressive-disclosure #129) measured **−30.5%**, not the recorded −65.2%: #129 rendered the symbol index as `path: path#Name1, path#Name2, …`, repeating the file path inside every anchor (already the line prefix), halving the win (median with_map 29.8k → 60.1k; 1-turn answers 10 → 8/20). Accuracy held 20/20. **Fix (#138, on main):** render bare `name`/`name@line` (anchor stays derivable as `path#name`); +a PARTIAL-index legend ("grep if your symbol isn't listed"). Compact names alone restored −65.4% **but** dropped accuracy to 18–19/20 (the cleaner index made the agent over-trust → guess on out-of-index answers like `capture-manager`, wrong 2/2); the legend restored grepping → **20/20 acc AND −65.4% tokens** (median 29,937). 4 billed passes + coverage.py.

**Competitive arm — atlas vs Aider repo-map (2026-06-21, equal-budget comprehension, claude-sonnet-4-6, pytest 8.2.0, N=1/arm).** First head-to-head vs a competitor's map at matched actual size. without_map **86,525 / 20-20**; **Aider** (aider 0.86.2 `--map-tokens 1024 --show-repo-map` = 1,942 tok) **59,452 / −31.3% / 20-20**; **atlas** (`--budget 2048` = 2,040 tok) **29,937 / −65.4% / 20-20**. All 20/20 accuracy; **atlas answers at ~half Aider's token cost.** Free answer-in-map coverage (the predictor): atlas **12/20** vs Aider **2/20** at ~2k; Aider only 5/20 even at its 2× budget overshoot (asked 2,048 → 4,126 actual). Aider spends budget on test/doc files + function-body snippets (`⋮`); atlas ranks + surfaces the symbol index. Method: Aider map injected via an `ATLAS_BIN` wrapper into comprehension.sh; matched ACTUAL tokens (not the budget knob) to isolate map quality. Caveat: N=1/arm (comprehension is low-variance; accuracy held across all arms).

**Competitive arm — atlas vs ponytail (2026-06-21, edit task `02-reuse-existing-utility`, claude-sonnet-4-6, pytest 8.2.0, N=1/arm, 15-turn cap).** ponytail (DietrichGebert/ponytail) is a behavioral "write less code" skill, not a map, so it can't enter the comprehension benchmark; the fair axis is a find-and-reuse edit task (use the existing `absolutepath` helper, don't reimplement). 4 arms, injected via `ATLAS_BIN` wrappers: **neither** reimplemented (FAIL), 1,969,820 explore tok, capped; **ponytail-only** (its 26-line skill) reimplemented (FAIL), 1,453,927 tok, capped; **atlas-only** reused (PASS), **662,121 tok, 7 turns**; **both** reused (PASS), 1,169,403 tok. **The structural map is the decisive factor: both map-having arms PASSED, both map-less arms FAILED** — ponytail's "reuse, don't rewrite" nudge can't help an agent find code it can't see, so it reimplements (the exact failure atlas prevents). atlas succeeds at ~half ponytail's token cost. Caveat: N=1, edit-task tokens are high-variance (60–140%) — the robust signal is the PASS/FAIL (reuse vs reimplement), not the exact counts. (results/run-20260621-09*.local.json)
