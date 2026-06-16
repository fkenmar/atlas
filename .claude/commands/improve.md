---
description: One iteration of the self-improvement loop (run continuously via /loop /improve)
---
Execute ONE iteration of the self-improvement loop, following docs/SELF_IMPROVEMENT.md exactly:

1. Read STATUS.md; pick exactly one item — unmet exit criterion first, else top NOW item. State the pick and the success metric for it in one line before touching anything.
2. Snapshot the before-stats the item can affect (tests/clippy always; pytest-map timing for perf; benchmark/history.md medians for ranking/budgeting/extraction/render changes).
3. Implement it — tests first where testable; conventions per CLAUDE.md. New dependency needed → STOP and ask (human gate).
4. Gate: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`. Never measure on a red tree.
5. Measure only what the change affects. Agent-behavior changes → `./benchmark/run.sh`, compare vs baseline.json and the latest benchmark/history.md row. Changes to map content or rendering (extraction/ranking/budgeting/render) → ALSO `./benchmark/comprehension.sh`; with-map accuracy below without-map accuracy is a hard regression no token win can buy back. Respect the $15/session benchmark spend guard.
6. Decide: improved/neutral-by-design → update STATUS.md, append the benchmark/history.md row (if measured), CHANGELOG line with the delta, commit (one commit, stats in message). Regressed → `git restore`, log the failed approach + numbers in ideas.md under "Tried and reverted".
7. Report one block: item, before → after, Δ%, keep/revert, suggested next pick.

Stop (end the loop instead of iterating) if: milestone exit criteria are all met, you hit a human gate, or two consecutive iterations on the same item reverted. Report the stop reason plainly.
