---
description: Run the agent benchmark, compare to baseline, update STATUS.md
---
Run the benchmark and report the delta:

1. Delegate to the **benchmark-runner** subagent: run `./benchmark/run.sh`, parse the newest `benchmark/results/run-*.local.json`, and compare against `benchmark/baseline.json` — **but only if their metric generations match**. A result is `schema_version: 3` with `metric` = the exploration-tokens-before-first-edit metric; the baseline must be `schema_version: 2` with a matching `metric`. If the baseline is the old whole-session proxy (schema 1, no `metric`), the comparison is INVALID — the subagent must report that the baseline needs re-recording (`run.sh --record-baseline`) and fall back to the **same-run** with_map vs without_map delta, which is always metric-consistent.
2. Present its delta table here: per-task exploration tokens and turns, with-map vs. baseline (or same-run), plus the aggregate vs. the ≥25% exploration-token-reduction target (PASS/FAIL). Medians are over **passing runs only**.
3. Update the "**Last benchmark result:**" line in STATUS.md with the date, aggregate delta, and pass/fail.
4. If any task regressed, list which ones and the subagent's findings about what they share — do not start fixing anything inside this command.

If the change being evaluated touches map content or rendering (extraction, ranking, budgeting, render), also run `./benchmark/comprehension.sh` and report the accuracy comparison — with-map accuracy below without-map is a hard fail (benchmark-protocol skill).
