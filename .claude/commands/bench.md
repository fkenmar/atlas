---
description: Run the agent benchmark, compare to baseline, update STATUS.md
---
Run the benchmark and report the delta:

1. Delegate to the **benchmark-runner** subagent: run `./benchmark/run.sh`, parse the newest `benchmark/results/run-*.local.json`, and compare against `benchmark/baseline.json`.
2. Present its delta table here: per-task exploration tokens and turns, with-map vs. baseline, plus the aggregate vs. the ≥25% exploration-token-reduction target (PASS/FAIL).
3. Update the "**Last benchmark result:**" line in STATUS.md with the date, aggregate delta, and pass/fail.
4. If any task regressed, list which ones and the subagent's findings about what they share — do not start fixing anything inside this command.

If run.sh still emits stub (null) results, report that the harness integration (an M0 exit criterion) hasn't landed and skip step 3's numbers — write "stub run, no signal" instead.
